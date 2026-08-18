#include <windows.h>

#include <iostream>
#include <string>
#include <vector>

namespace {

constexpr wchar_t kEncoderPath[] =
    L"SOFTWARE\\Microsoft\\Multimedia\\Audio\\Spatial\\Encoder";
constexpr wchar_t kSpatialEndpointPath[] =
    L"SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\MMDevices\\SpatialAudioEndpoint";

std::wstring Win32Message(LONG code) {
    wchar_t buffer[512] = {};
    const DWORD count = FormatMessageW(
        FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
        nullptr,
        static_cast<DWORD>(code),
        0,
        buffer,
        static_cast<DWORD>(std::size(buffer)),
        nullptr);
    if (count == 0) {
        return L"unknown";
    }
    std::wstring message(buffer, count);
    while (!message.empty() &&
           (message.back() == L'\r' || message.back() == L'\n' || message.back() == L' ')) {
        message.pop_back();
    }
    return message;
}

std::wstring ReadStringValue(HKEY key, const wchar_t* valueName) {
    DWORD bytes = 0;
    LONG result = RegGetValueW(
        key,
        nullptr,
        valueName,
        RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ,
        nullptr,
        nullptr,
        &bytes);
    if (result != ERROR_SUCCESS || bytes == 0) {
        return {};
    }

    std::vector<wchar_t> value((bytes / sizeof(wchar_t)) + 1, L'\0');
    result = RegGetValueW(
        key,
        nullptr,
        valueName,
        RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ,
        nullptr,
        value.data(),
        &bytes);
    if (result != ERROR_SUCCESS) {
        return {};
    }
    return value.data();
}

void PrintStringField(const std::wstring& prefix, const wchar_t* field, const std::wstring& value) {
    std::wcout << prefix << L"." << field << L"=";
    if (value.empty()) {
        std::wcout << L"<missing>";
    } else {
        std::wcout << value;
    }
    std::wcout << L"\n";
}

LONG OpenReadOnly64(const wchar_t* path, HKEY* key) {
    return RegOpenKeyExW(
        HKEY_LOCAL_MACHINE,
        path,
        0,
        KEY_READ | KEY_WOW64_64KEY,
        key);
}

void ProbeEncoderRegistry() {
    std::wcout << L"encoder.path=HKLM\\" << kEncoderPath << L"\n";

    HKEY root = nullptr;
    const LONG openResult = OpenReadOnly64(kEncoderPath, &root);
    if (openResult != ERROR_SUCCESS) {
        std::wcout << L"encoder.status=unavailable\n";
        std::wcout << L"encoder.error=" << openResult << L":" << Win32Message(openResult) << L"\n";
        return;
    }

    std::wcout << L"encoder.status=available\n";

    DWORD subkeyCount = 0;
    DWORD maxSubkeyLength = 0;
    const LONG infoResult = RegQueryInfoKeyW(
        root,
        nullptr,
        nullptr,
        nullptr,
        &subkeyCount,
        &maxSubkeyLength,
        nullptr,
        nullptr,
        nullptr,
        nullptr,
        nullptr,
        nullptr);
    if (infoResult != ERROR_SUCCESS) {
        std::wcout << L"encoder.enumeration_error=" << infoResult << L":" << Win32Message(infoResult) << L"\n";
        RegCloseKey(root);
        return;
    }

    std::wcout << L"encoder.count=" << subkeyCount << L"\n";
    std::vector<wchar_t> name(maxSubkeyLength + 2, L'\0');

    for (DWORD index = 0; index < subkeyCount; ++index) {
        DWORD nameLength = static_cast<DWORD>(name.size());
        FILETIME lastWrite = {};
        const LONG enumResult = RegEnumKeyExW(
            root,
            index,
            name.data(),
            &nameLength,
            nullptr,
            nullptr,
            nullptr,
            &lastWrite);
        if (enumResult != ERROR_SUCCESS) {
            std::wcout << L"encoder[" << index << L"].error="
                       << enumResult << L":" << Win32Message(enumResult) << L"\n";
            continue;
        }

        const std::wstring formatGuid(name.data(), nameLength);
        const std::wstring prefix = L"encoder[" + std::to_wstring(index) + L"]";
        std::wcout << prefix << L".format_guid=" << formatGuid << L"\n";

        HKEY provider = nullptr;
        const LONG providerResult = RegOpenKeyExW(
            root,
            formatGuid.c_str(),
            0,
            KEY_READ | KEY_WOW64_64KEY,
            &provider);
        if (providerResult != ERROR_SUCCESS) {
            std::wcout << prefix << L".open_error="
                       << providerResult << L":" << Win32Message(providerResult) << L"\n";
            continue;
        }

        PrintStringField(prefix, L"display_name", ReadStringValue(provider, nullptr));
        PrintStringField(prefix, L"clsid", ReadStringValue(provider, L"CLSID"));
        PrintStringField(prefix, L"icon_path", ReadStringValue(provider, L"IconPath"));
        RegCloseKey(provider);
    }

    RegCloseKey(root);
}

void ProbeSpatialEndpointRegistry() {
    std::wcout << L"spatial_endpoint.path=HKLM\\" << kSpatialEndpointPath << L"\n";

    HKEY root = nullptr;
    const LONG openResult = OpenReadOnly64(kSpatialEndpointPath, &root);
    if (openResult != ERROR_SUCCESS) {
        std::wcout << L"spatial_endpoint.status=unavailable\n";
        std::wcout << L"spatial_endpoint.error=" << openResult << L":" << Win32Message(openResult) << L"\n";
        return;
    }

    std::wcout << L"spatial_endpoint.status=available\n";

    DWORD subkeyCount = 0;
    DWORD maxSubkeyLength = 0;
    const LONG infoResult = RegQueryInfoKeyW(
        root,
        nullptr,
        nullptr,
        nullptr,
        &subkeyCount,
        &maxSubkeyLength,
        nullptr,
        nullptr,
        nullptr,
        nullptr,
        nullptr,
        nullptr);
    if (infoResult != ERROR_SUCCESS) {
        std::wcout << L"spatial_endpoint.enumeration_error="
                   << infoResult << L":" << Win32Message(infoResult) << L"\n";
        RegCloseKey(root);
        return;
    }

    std::wcout << L"spatial_endpoint.subkey_count=" << subkeyCount << L"\n";
    std::vector<wchar_t> name(maxSubkeyLength + 2, L'\0');
    for (DWORD index = 0; index < subkeyCount; ++index) {
        DWORD nameLength = static_cast<DWORD>(name.size());
        FILETIME lastWrite = {};
        const LONG enumResult = RegEnumKeyExW(
            root,
            index,
            name.data(),
            &nameLength,
            nullptr,
            nullptr,
            nullptr,
            &lastWrite);
        if (enumResult == ERROR_SUCCESS) {
            std::wcout << L"spatial_endpoint.subkey[" << index << L"]="
                       << std::wstring(name.data(), nameLength) << L"\n";
        } else {
            std::wcout << L"spatial_endpoint.subkey[" << index << L"].error="
                       << enumResult << L":" << Win32Message(enumResult) << L"\n";
        }
    }

    RegCloseKey(root);
}

} // namespace

int wmain() {
    std::wcout << L"probe=omniphony_spatial_provider\n";
    std::wcout << L"mode=read_only_observation\n";
    std::wcout << L"source_truth=undocumented_registry_surface_not_public_api_contract\n";
    ProbeEncoderRegistry();
    ProbeSpatialEndpointRegistry();
    return 0;
}
