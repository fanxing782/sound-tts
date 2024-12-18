#include "sound_tts.h"

int main()
{
    SetConsoleOutputCP(CP_UTF8);
    // 加载dll
    //SetDllDirectory(L"");
    HMODULE hModule = LoadLibrary(L"sound_tts.dll");
    if (!hModule) {
        std::cerr << "Failed to load DLL!" << std::endl;
        return 1;
    }


    // 初始化
    sound_tts_init init = (sound_tts_init)GetProcAddress(hModule, "sound_tts_init");
    // 加载设备列表
    sound_tts_get_devices get_devices = (sound_tts_get_devices)GetProcAddress(hModule, "sound_tts_get_devices");
    // 释放设备列表缓存
    sound_tts_free_device_list  free_device_list = (sound_tts_free_device_list)GetProcAddress(hModule, "sound_tts_free_device_list");
    // 设备是否存在
    sound_tts_device_is_exits device_is_exist = (sound_tts_device_is_exits)GetProcAddress(hModule, "sound_tts_device_is_exist");
    // 播放语音
    sound_tts_speak speak = (sound_tts_speak)GetProcAddress(hModule, "sound_tts_speak");
    // 停止播放
    sound_tts_stop stop = (sound_tts_stop)GetProcAddress(hModule, "sound_tts_stop");


    if (!init || !get_devices || !free_device_list || !device_is_exist || !speak || !stop) {
        std::cout << "Failed to method loading!" <<std::endl;
        return -1;
    }

    init();

    const SoundTtsDeviceList* list = get_devices();

    if (list) {
        for (size_t i = 0; i < list->len; ++i) {
            std::cout << "Device " << i + 1 << ": " << list->ptr[i] << std::endl;
        }
        free_device_list(list);
    }

    std::wstring device_name = L"VG27AQML1A (NVIDIA High Definition Audio)";
    bool exist =  device_is_exist(UTF8(device_name));
    if (exist){
        speak(UTF8(L"测试播放内容"), 0, 1, UTF8(device_name), true);
    }

    std::this_thread::sleep_for(std::chrono::seconds(10));
    stop(UTF8(device_name));
}

