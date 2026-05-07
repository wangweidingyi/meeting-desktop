#include "system_audio_bridge.h"

#import <ApplicationServices/ApplicationServices.h>
#import <CoreAudio/CoreAudio.h>
#import <CoreMedia/CoreMedia.h>
#import <Foundation/Foundation.h>
#import <ScreenCaptureKit/ScreenCaptureKit.h>

#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace {

uint64_t current_unix_ms() {
    using namespace std::chrono;
    return duration_cast<milliseconds>(system_clock::now().time_since_epoch()).count();
}

std::string fourcc(OSStatus status) {
    char value[5] = {
        static_cast<char>((status >> 24) & 0xff),
        static_cast<char>((status >> 16) & 0xff),
        static_cast<char>((status >> 8) & 0xff),
        static_cast<char>(status & 0xff),
        '\0',
    };

    for (int index = 0; index < 4; ++index) {
        if (value[index] < 32 || value[index] > 126) {
            return "";
        }
    }

    return std::string(value, 4);
}

std::string osstatus_message(const char *operation, OSStatus status) {
    char buffer[192];
    std::string status_fourcc = fourcc(status);
    if (status_fourcc.empty()) {
        std::snprintf(buffer, sizeof(buffer), "%s failed with OSStatus %d", operation, status);
    } else {
        std::snprintf(
            buffer,
            sizeof(buffer),
            "%s failed with OSStatus %d ('%s')",
            operation,
            status,
            status_fourcc.c_str());
    }
    return std::string(buffer);
}

std::string ns_error_message(const char *operation, NSError *error) {
    if (error == nil) {
        return std::string(operation) + " failed";
    }

    return
        std::string(operation) + " failed [" +
        std::string([[error domain] UTF8String]) + " " +
        std::to_string(static_cast<long long>(error.code)) + "]: " +
        std::string([[error localizedDescription] UTF8String]);
}

void set_error(char *error_buffer, size_t error_buffer_len, const std::string &message) {
    if (error_buffer == nullptr || error_buffer_len == 0) {
        return;
    }

    std::snprintf(error_buffer, error_buffer_len, "%s", message.c_str());
}

bool wait_for_dispatch_semaphore(dispatch_semaphore_t semaphore, int64_t timeout_ms) {
    return dispatch_semaphore_wait(
               semaphore,
               dispatch_time(DISPATCH_TIME_NOW, timeout_ms * NSEC_PER_MSEC)) == 0;
}

SCDisplay *select_capture_display(SCShareableContent *shareable_content) {
    if (shareable_content == nil || shareable_content.displays.count == 0) {
        return nil;
    }

    CGDirectDisplayID main_display_id = CGMainDisplayID();
    for (SCDisplay *display in shareable_content.displays) {
        if (display.displayID == main_display_id) {
            return display;
        }
    }

    return shareable_content.displays.firstObject;
}

bool extract_interleaved_float_samples(
    CMSampleBufferRef sample_buffer,
    std::vector<float> &out_samples,
    uint32_t &out_sample_rate_hz,
    uint16_t &out_channels,
    std::string &error) {
    if (sample_buffer == nullptr || !CMSampleBufferIsValid(sample_buffer)) {
        error = "ScreenCaptureKit delivered an invalid audio sample buffer";
        return false;
    }

    CMFormatDescriptionRef format_description = CMSampleBufferGetFormatDescription(sample_buffer);
    if (format_description == nullptr) {
        error = "ScreenCaptureKit audio sample buffer has no format description";
        return false;
    }

    const AudioStreamBasicDescription *format =
        CMAudioFormatDescriptionGetStreamBasicDescription(
            static_cast<CMAudioFormatDescriptionRef>(format_description));
    if (format == nullptr) {
        error = "ScreenCaptureKit audio sample buffer has no stream format";
        return false;
    }

    if (format->mChannelsPerFrame == 0 || format->mSampleRate <= 0) {
        error = "ScreenCaptureKit audio sample buffer returned an invalid stream format";
        return false;
    }

    const CMItemCount frame_count = CMSampleBufferGetNumSamples(sample_buffer);
    if (frame_count <= 0) {
        return false;
    }

    size_t buffer_list_size = 0;
    OSStatus status = CMSampleBufferGetAudioBufferListWithRetainedBlockBuffer(
        sample_buffer,
        &buffer_list_size,
        nullptr,
        0,
        kCFAllocatorDefault,
        kCFAllocatorDefault,
        kCMSampleBufferFlag_AudioBufferList_Assure16ByteAlignment,
        nullptr);
    if (status != noErr || buffer_list_size == 0) {
        error = osstatus_message(
            "CMSampleBufferGetAudioBufferListWithRetainedBlockBuffer(size query)",
            status);
        return false;
    }

    std::vector<uint8_t> buffer_list_storage(buffer_list_size);
    auto *buffer_list = reinterpret_cast<AudioBufferList *>(buffer_list_storage.data());
    CMBlockBufferRef retained_block_buffer = nullptr;
    status = CMSampleBufferGetAudioBufferListWithRetainedBlockBuffer(
        sample_buffer,
        nullptr,
        buffer_list,
        buffer_list_size,
        kCFAllocatorDefault,
        kCFAllocatorDefault,
        kCMSampleBufferFlag_AudioBufferList_Assure16ByteAlignment,
        &retained_block_buffer);
    if (status != noErr) {
        error = osstatus_message(
            "CMSampleBufferGetAudioBufferListWithRetainedBlockBuffer",
            status);
        return false;
    }

    const bool is_linear_pcm = format->mFormatID == kAudioFormatLinearPCM;
    const bool is_float32 =
        is_linear_pcm &&
        (format->mFormatFlags & kAudioFormatFlagIsFloat) != 0 &&
        format->mBitsPerChannel == 32;
    const bool is_int16 =
        is_linear_pcm &&
        (format->mFormatFlags & kAudioFormatFlagIsSignedInteger) != 0 &&
        format->mBitsPerChannel == 16;
    const bool is_non_interleaved =
        (format->mFormatFlags & kAudioFormatFlagIsNonInterleaved) != 0;
    const UInt32 channels = format->mChannelsPerFrame;

    out_sample_rate_hz = static_cast<uint32_t>(std::llround(format->mSampleRate));
    out_channels = static_cast<uint16_t>(std::min<UInt32>(channels, UINT16_MAX));
    out_samples.clear();
    out_samples.reserve(static_cast<size_t>(frame_count) * channels);

    if (!is_float32 && !is_int16) {
        if (retained_block_buffer != nullptr) {
            CFRelease(retained_block_buffer);
        }
        error = "ScreenCaptureKit audio sample buffer is not float32 or int16 PCM";
        return false;
    }

    if (is_non_interleaved) {
        if (buffer_list->mNumberBuffers < channels) {
            if (retained_block_buffer != nullptr) {
                CFRelease(retained_block_buffer);
            }
            error = "ScreenCaptureKit returned fewer audio buffers than channel count";
            return false;
        }

        for (CMItemCount frame_index = 0; frame_index < frame_count; ++frame_index) {
            for (UInt32 channel_index = 0; channel_index < channels; ++channel_index) {
                const AudioBuffer &buffer = buffer_list->mBuffers[channel_index];
                if (buffer.mData == nullptr) {
                    out_samples.push_back(0.0f);
                    continue;
                }

                if (is_float32) {
                    const float *data = static_cast<const float *>(buffer.mData);
                    out_samples.push_back(data[frame_index]);
                } else {
                    const int16_t *data = static_cast<const int16_t *>(buffer.mData);
                    out_samples.push_back(
                        static_cast<float>(data[frame_index]) / static_cast<float>(INT16_MAX));
                }
            }
        }
    } else {
        if (buffer_list->mNumberBuffers == 0 || buffer_list->mBuffers[0].mData == nullptr) {
            if (retained_block_buffer != nullptr) {
                CFRelease(retained_block_buffer);
            }
            error = "ScreenCaptureKit returned no interleaved audio buffer data";
            return false;
        }

        const size_t sample_count = static_cast<size_t>(frame_count) * channels;
        out_samples.resize(sample_count);
        if (is_float32) {
            const float *data = static_cast<const float *>(buffer_list->mBuffers[0].mData);
            std::copy(data, data + sample_count, out_samples.begin());
        } else {
            const int16_t *data = static_cast<const int16_t *>(buffer_list->mBuffers[0].mData);
            for (size_t sample_index = 0; sample_index < sample_count; ++sample_index) {
                out_samples[sample_index] =
                    static_cast<float>(data[sample_index]) / static_cast<float>(INT16_MAX);
            }
        }
    }

    if (retained_block_buffer != nullptr) {
        CFRelease(retained_block_buffer);
    }

    return !out_samples.empty();
}

}  // namespace

API_AVAILABLE(macos(13.0))
@interface MeetingSystemAudioCaptureSession : NSObject <SCStreamOutput, SCStreamDelegate>

- (instancetype)initWithCallback:(meeting_system_audio_callback)callback
                        userData:(void *)userData;
- (BOOL)startWithErrorMessage:(std::string &)errorMessage;
- (void)stop;

@end

API_AVAILABLE(macos(13.0))
@implementation MeetingSystemAudioCaptureSession {
    meeting_system_audio_callback _callback;
    void *_userData;
    SCStream *_stream;
    dispatch_queue_t _sampleQueue;
    BOOL _started;
}

- (instancetype)initWithCallback:(meeting_system_audio_callback)callback
                        userData:(void *)userData {
    self = [super init];
    if (self != nil) {
        _callback = callback;
        _userData = userData;
    }
    return self;
}

- (SCDisplay *)loadDisplayWithErrorMessage:(std::string &)errorMessage {
    __block SCShareableContent *shareableContent = nil;
    __block NSError *shareableContentError = nil;
    dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);

    [SCShareableContent getShareableContentWithCompletionHandler:^(
        SCShareableContent * _Nullable content,
        NSError * _Nullable error
    ) {
        shareableContent = content;
        shareableContentError = error;
        dispatch_semaphore_signal(semaphore);
    }];

    if (!wait_for_dispatch_semaphore(semaphore, 10'000)) {
        errorMessage = "timed out waiting for ScreenCaptureKit shareable content";
        return nil;
    }

    if (shareableContentError != nil) {
        if ([shareableContentError.domain isEqualToString:SCStreamErrorDomain] &&
            shareableContentError.code == SCStreamErrorUserDeclined) {
            errorMessage = "screen recording permission denied";
        } else {
            errorMessage =
                ns_error_message("SCShareableContent getShareableContent", shareableContentError);
        }
        return nil;
    }

    SCDisplay *display = select_capture_display(shareableContent);
    if (display == nil) {
        errorMessage = "ScreenCaptureKit returned no displays to capture";
        return nil;
    }

    return display;
}

- (BOOL)startWithErrorMessage:(std::string &)errorMessage {
    if (_callback == nullptr) {
        errorMessage = "callback must not be null";
        return NO;
    }

    if (!CGPreflightScreenCaptureAccess() && !CGRequestScreenCaptureAccess()) {
        errorMessage = "screen recording permission denied";
        return NO;
    }

    SCDisplay *display = [self loadDisplayWithErrorMessage:errorMessage];
    if (display == nil) {
        return NO;
    }

    SCContentFilter *filter =
        [[SCContentFilter alloc] initWithDisplay:display
                            excludingApplications:@[]
                                 exceptingWindows:@[]];
    SCStreamConfiguration *configuration = [[SCStreamConfiguration alloc] init];
    configuration.width = static_cast<size_t>(std::max<NSInteger>(display.width, 1));
    configuration.height = static_cast<size_t>(std::max<NSInteger>(display.height, 1));
    configuration.minimumFrameInterval = CMTimeMake(1, 60);
    configuration.queueDepth = 3;
    configuration.showsCursor = NO;
    configuration.capturesAudio = YES;
    configuration.sampleRate = 48'000;
    configuration.channelCount = 2;
    configuration.excludesCurrentProcessAudio = YES;

    _sampleQueue = dispatch_queue_create("com.cxc.meeting.system-audio", DISPATCH_QUEUE_SERIAL);
    _stream = [[SCStream alloc] initWithFilter:filter configuration:configuration delegate:self];

    NSError *addOutputError = nil;
    if (![_stream addStreamOutput:self
                             type:SCStreamOutputTypeAudio
               sampleHandlerQueue:_sampleQueue
                            error:&addOutputError]) {
        errorMessage = ns_error_message("SCStream addStreamOutput", addOutputError);
        _stream = nil;
        _sampleQueue = nil;
        return NO;
    }

    __block NSError *startError = nil;
    dispatch_semaphore_t startSemaphore = dispatch_semaphore_create(0);
    [_stream startCaptureWithCompletionHandler:^(NSError * _Nullable error) {
        startError = error;
        dispatch_semaphore_signal(startSemaphore);
    }];

    if (!wait_for_dispatch_semaphore(startSemaphore, 10'000)) {
        errorMessage = "timed out waiting for ScreenCaptureKit capture start";
        [self stop];
        return NO;
    }

    if (startError != nil) {
        if ([startError.domain isEqualToString:SCStreamErrorDomain] &&
            startError.code == SCStreamErrorUserDeclined) {
            errorMessage = "screen recording permission denied";
        } else {
            errorMessage = ns_error_message("SCStream startCapture", startError);
        }
        [self stop];
        return NO;
    }

    _started = YES;
    return YES;
}

- (void)stop {
    SCStream *stream = _stream;
    _stream = nil;
    _started = NO;

    if (stream == nil) {
        _sampleQueue = nil;
        return;
    }

    NSError *removeOutputError = nil;
    [stream removeStreamOutput:self type:SCStreamOutputTypeAudio error:&removeOutputError];

    dispatch_semaphore_t stopSemaphore = dispatch_semaphore_create(0);
    [stream stopCaptureWithCompletionHandler:^(NSError * _Nullable error) {
        if (error != nil) {
            NSLog(@"Meeting system audio stop: %@", error.localizedDescription);
        }
        dispatch_semaphore_signal(stopSemaphore);
    }];
    wait_for_dispatch_semaphore(stopSemaphore, 5'000);
    _sampleQueue = nil;
}

- (void)stream:(SCStream *)stream
didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer
        ofType:(SCStreamOutputType)type {
    if (!_started || type != SCStreamOutputTypeAudio) {
        return;
    }

    std::vector<float> samples;
    uint32_t sampleRateHz = 0;
    uint16_t channels = 0;
    std::string error;
    if (!extract_interleaved_float_samples(
            sampleBuffer,
            samples,
            sampleRateHz,
            channels,
            error)) {
        if (!error.empty()) {
            NSLog(@"Meeting system audio sample conversion: %s", error.c_str());
        }
        return;
    }

    if (samples.empty()) {
        return;
    }

    _callback(
        _userData,
        current_unix_ms(),
        samples.data(),
        samples.size(),
        sampleRateHz,
        channels);
}

- (void)stream:(SCStream *)stream didStopWithError:(NSError *)error {
    if (error != nil) {
        NSLog(@"Meeting system audio stream stopped: %@", error.localizedDescription);
    }
}

@end

extern "C" bool meeting_system_audio_start(
    meeting_system_audio_callback callback,
    void *user_data,
    void **out_handle,
    char *error_buffer,
    size_t error_buffer_len) {
    if (out_handle == nullptr) {
        set_error(error_buffer, error_buffer_len, "out_handle must not be null");
        return false;
    }
    *out_handle = nullptr;

    if (callback == nullptr) {
        set_error(error_buffer, error_buffer_len, "callback must not be null");
        return false;
    }

    @autoreleasepool {
        if (@available(macOS 13.0, *)) {
            MeetingSystemAudioCaptureSession *session =
                [[MeetingSystemAudioCaptureSession alloc] initWithCallback:callback
                                                                  userData:user_data];
            std::string error;
            if (![session startWithErrorMessage:error]) {
                set_error(error_buffer, error_buffer_len, error);
                return false;
            }

            *out_handle = (__bridge_retained void *)session;
            return true;
        }

        set_error(
            error_buffer,
            error_buffer_len,
            "macOS system audio capture requires ScreenCaptureKit audio capture");
        return false;
    }
}

extern "C" void meeting_system_audio_stop(void *raw_handle) {
    if (raw_handle == nullptr) {
        return;
    }

    @autoreleasepool {
        MeetingSystemAudioCaptureSession *session =
            (__bridge_transfer MeetingSystemAudioCaptureSession *)raw_handle;
        [session stop];
    }
}
