#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef void (*meeting_system_audio_callback)(
    void *user_data,
    uint64_t started_at_ms,
    const float *samples,
    size_t sample_count,
    uint32_t sample_rate_hz,
    uint16_t channels);

#ifdef __cplusplus
extern "C" {
#endif

bool meeting_system_audio_start(
    meeting_system_audio_callback callback,
    void *user_data,
    void **out_handle,
    char *error_buffer,
    size_t error_buffer_len);

void meeting_system_audio_stop(void *handle);

#ifdef __cplusplus
}
#endif
