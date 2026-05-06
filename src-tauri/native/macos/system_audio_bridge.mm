#include "system_audio_bridge.h"

#import <CoreAudio/AudioHardware.h>
#import <CoreAudio/AudioHardwareTapping.h>
#import <CoreAudio/CATapDescription.h>
#import <Foundation/Foundation.h>

#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace {

struct MeetingSystemAudioHandle {
    meeting_system_audio_callback callback = nullptr;
    void *user_data = nullptr;
    AudioObjectID tap_id = kAudioObjectUnknown;
    AudioObjectID aggregate_device_id = kAudioObjectUnknown;
    AudioDeviceIOProcID io_proc_id = nullptr;
    uint32_t sample_rate_hz = 0;
    uint16_t channels = 0;
    bool started = false;
};

NSString *dictionary_key(const char *key) {
    return [NSString stringWithUTF8String:key];
}

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
    char buffer[160];
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

void set_error(char *error_buffer, size_t error_buffer_len, const std::string &message) {
    if (error_buffer == nullptr || error_buffer_len == 0) {
        return;
    }

    std::snprintf(error_buffer, error_buffer_len, "%s", message.c_str());
}

bool get_tap_uid(AudioObjectID tap_id, NSString **out_uid, std::string &error) {
    CFStringRef tap_uid = nullptr;
    UInt32 property_size = sizeof(tap_uid);
    AudioObjectPropertyAddress address = {
        kAudioTapPropertyUID,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    };

    OSStatus status = AudioObjectGetPropertyData(
        tap_id,
        &address,
        0,
        nullptr,
        &property_size,
        &tap_uid);
    if (status != noErr) {
        error = osstatus_message("AudioObjectGetPropertyData(kAudioTapPropertyUID)", status);
        return false;
    }
    if (tap_uid == nullptr) {
        error = "AudioObjectGetPropertyData(kAudioTapPropertyUID) returned no UID";
        return false;
    }

    *out_uid = CFBridgingRelease(tap_uid);
    return true;
}

bool get_tap_format(
    AudioObjectID tap_id,
    AudioStreamBasicDescription &format,
    std::string &error) {
    UInt32 property_size = sizeof(format);
    AudioObjectPropertyAddress address = {
        kAudioTapPropertyFormat,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    };

    OSStatus status = AudioObjectGetPropertyData(
        tap_id,
        &address,
        0,
        nullptr,
        &property_size,
        &format);
    if (status != noErr) {
        error = osstatus_message("AudioObjectGetPropertyData(kAudioTapPropertyFormat)", status);
        return false;
    }

    const bool is_pcm = format.mFormatID == kAudioFormatLinearPCM;
    const bool is_float = (format.mFormatFlags & kAudioFormatFlagIsFloat) != 0;
    if (!is_pcm || !is_float || format.mBitsPerChannel != 32) {
        error = "Core Audio system tap format is not 32-bit float PCM";
        return false;
    }
    if (format.mSampleRate <= 0 || format.mChannelsPerFrame == 0) {
        error = "Core Audio system tap returned an invalid stream format";
        return false;
    }

    return true;
}

OSStatus system_audio_io_proc(
    AudioObjectID,
    const AudioTimeStamp *,
    const AudioBufferList *in_input_data,
    const AudioTimeStamp *,
    AudioBufferList *,
    const AudioTimeStamp *,
    void *client_data) {
    auto *handle = static_cast<MeetingSystemAudioHandle *>(client_data);
    if (handle == nullptr || handle->callback == nullptr || in_input_data == nullptr) {
        return noErr;
    }

    if (in_input_data->mNumberBuffers == 0) {
        return noErr;
    }

    if (in_input_data->mNumberBuffers == 1) {
        const AudioBuffer &buffer = in_input_data->mBuffers[0];
        if (buffer.mData == nullptr || buffer.mDataByteSize == 0) {
            return noErr;
        }

        const uint16_t channels = buffer.mNumberChannels > 0
            ? static_cast<uint16_t>(std::min<UInt32>(buffer.mNumberChannels, UINT16_MAX))
            : handle->channels;
        const size_t sample_count = buffer.mDataByteSize / sizeof(float);
        handle->callback(
            handle->user_data,
            current_unix_ms(),
            static_cast<const float *>(buffer.mData),
            sample_count,
            handle->sample_rate_hz,
            channels);
        return noErr;
    }

    const UInt32 buffer_count = in_input_data->mNumberBuffers;
    const UInt32 channels = std::min<UInt32>(buffer_count, UINT16_MAX);
    size_t frame_count = SIZE_MAX;
    for (UInt32 buffer_index = 0; buffer_index < buffer_count; ++buffer_index) {
        const AudioBuffer &buffer = in_input_data->mBuffers[buffer_index];
        if (buffer.mData == nullptr) {
            continue;
        }
        frame_count = std::min(frame_count, static_cast<size_t>(buffer.mDataByteSize / sizeof(float)));
    }
    if (frame_count == SIZE_MAX || frame_count == 0) {
        return noErr;
    }

    std::vector<float> interleaved;
    interleaved.reserve(frame_count * channels);
    for (size_t frame_index = 0; frame_index < frame_count; ++frame_index) {
        for (UInt32 buffer_index = 0; buffer_index < channels; ++buffer_index) {
            const AudioBuffer &buffer = in_input_data->mBuffers[buffer_index];
            const float *data = static_cast<const float *>(buffer.mData);
            interleaved.push_back(data == nullptr ? 0.0f : data[frame_index]);
        }
    }

    handle->callback(
        handle->user_data,
        current_unix_ms(),
        interleaved.data(),
        interleaved.size(),
        handle->sample_rate_hz,
        static_cast<uint16_t>(channels));
    return noErr;
}

}  // namespace

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
        if (@available(macOS 14.2, *)) {
            auto *handle = new MeetingSystemAudioHandle();
            handle->callback = callback;
            handle->user_data = user_data;

            CATapDescription *tap_description =
                [[CATapDescription alloc] initStereoGlobalTapButExcludeProcesses:@[]];
            tap_description.name = @"Meeting System Audio";
            tap_description.privateTap = YES;
            tap_description.muteBehavior = CATapUnmuted;

            OSStatus status =
                AudioHardwareCreateProcessTap(tap_description, &handle->tap_id);
            if (status != noErr) {
                set_error(
                    error_buffer,
                    error_buffer_len,
                    osstatus_message("AudioHardwareCreateProcessTap", status));
                delete handle;
                return false;
            }

            std::string error;
            NSString *tap_uid = nil;
            if (!get_tap_uid(handle->tap_id, &tap_uid, error)) {
                set_error(error_buffer, error_buffer_len, error);
                meeting_system_audio_stop(handle);
                return false;
            }

            AudioStreamBasicDescription format = {};
            if (!get_tap_format(handle->tap_id, format, error)) {
                set_error(error_buffer, error_buffer_len, error);
                meeting_system_audio_stop(handle);
                return false;
            }
            handle->sample_rate_hz = static_cast<uint32_t>(std::llround(format.mSampleRate));
            handle->channels =
                static_cast<uint16_t>(std::min<UInt32>(format.mChannelsPerFrame, UINT16_MAX));

            NSString *aggregate_uid = [NSString stringWithFormat:
                @"com.cxc.meeting.system-audio.%@",
                [[NSUUID UUID] UUIDString]];
            NSDictionary *tap_entry = @{
                dictionary_key(kAudioSubTapUIDKey): tap_uid,
            };
            NSDictionary *aggregate_description = @{
                dictionary_key(kAudioAggregateDeviceUIDKey): aggregate_uid,
                dictionary_key(kAudioAggregateDeviceNameKey): @"Meeting System Audio",
                dictionary_key(kAudioAggregateDeviceIsPrivateKey): @YES,
                dictionary_key(kAudioAggregateDeviceTapListKey): @[ tap_entry ],
                dictionary_key(kAudioAggregateDeviceTapAutoStartKey): @YES,
            };

            status = AudioHardwareCreateAggregateDevice(
                (__bridge CFDictionaryRef)aggregate_description,
                &handle->aggregate_device_id);
            if (status != noErr) {
                set_error(
                    error_buffer,
                    error_buffer_len,
                    osstatus_message("AudioHardwareCreateAggregateDevice", status));
                meeting_system_audio_stop(handle);
                return false;
            }

            status = AudioDeviceCreateIOProcID(
                handle->aggregate_device_id,
                system_audio_io_proc,
                handle,
                &handle->io_proc_id);
            if (status != noErr) {
                set_error(
                    error_buffer,
                    error_buffer_len,
                    osstatus_message("AudioDeviceCreateIOProcID", status));
                meeting_system_audio_stop(handle);
                return false;
            }

            status = AudioDeviceStart(handle->aggregate_device_id, handle->io_proc_id);
            if (status != noErr) {
                set_error(
                    error_buffer,
                    error_buffer_len,
                    osstatus_message("AudioDeviceStart", status));
                meeting_system_audio_stop(handle);
                return false;
            }

            handle->started = true;
            *out_handle = handle;
            return true;
        } else {
            set_error(
                error_buffer,
                error_buffer_len,
                "macOS system audio capture requires macOS 14.2 or newer");
            return false;
        }
    }
}

extern "C" void meeting_system_audio_stop(void *raw_handle) {
    if (raw_handle == nullptr) {
        return;
    }

    auto *handle = static_cast<MeetingSystemAudioHandle *>(raw_handle);
    if (handle->started && handle->aggregate_device_id != kAudioObjectUnknown) {
        AudioDeviceStop(handle->aggregate_device_id, handle->io_proc_id);
        handle->started = false;
    }
    if (handle->io_proc_id != nullptr && handle->aggregate_device_id != kAudioObjectUnknown) {
        AudioDeviceDestroyIOProcID(handle->aggregate_device_id, handle->io_proc_id);
        handle->io_proc_id = nullptr;
    }
    if (handle->aggregate_device_id != kAudioObjectUnknown) {
        AudioHardwareDestroyAggregateDevice(handle->aggregate_device_id);
        handle->aggregate_device_id = kAudioObjectUnknown;
    }
    if (handle->tap_id != kAudioObjectUnknown) {
        if (@available(macOS 14.2, *)) {
            AudioHardwareDestroyProcessTap(handle->tap_id);
        }
        handle->tap_id = kAudioObjectUnknown;
    }

    delete handle;
}
