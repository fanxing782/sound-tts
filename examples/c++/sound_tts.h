#include <Windows.h>
#include <iostream>
#include <chrono>
#include <thread>
#include <string>
#include <codecvt>

struct SoundTtsDeviceList {
    const char** ptr;
    size_t len;
};

#define UTF8(wstr) (std::wstring_convert<std::codecvt_utf8<wchar_t>>().to_bytes(wstr).c_str())
typedef void (*sound_tts_init)();
typedef const SoundTtsDeviceList* (*sound_tts_get_devices)();
typedef void (*sound_tts_free_device_list)(const SoundTtsDeviceList* device_list);
typedef bool (*sound_tts_device_is_exits)(const char* device_name);
typedef void (*sound_tts_speak)(const char* str, uint64_t play_count, uint64_t play_interval, const char* device_name, bool interrupt);
typedef bool (*sound_tts_stop)(const char* device_name);


