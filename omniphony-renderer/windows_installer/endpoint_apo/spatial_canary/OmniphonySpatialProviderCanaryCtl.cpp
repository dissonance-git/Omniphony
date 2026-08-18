#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <objbase.h>

#include <iostream>
#include <string>
#include <vector>

namespace {

constexpr GUID kProviderClsid = {
    0x7aee0f13, 0x1f6b, 0x4d83,
    {0x9f, 0x6d, 0x6c, 0x9c, 0x0e, 0x33, 0xa1, 0x51}
};

constexpr GUID kFormatGuid = {
    0x3dbff1af, 0x0fc6, 0x4a32,
    {0x82, 0x89, 0x5e, 0x65, 0x2c, 0x98, 0x7d, 0x92}
};

constexpr GUID kSelftestIid = {
    0x8875a4e2, 0x12e9, 0x49c3,
    {0xa8, 0x19, 0x6d, 0xa3, 0x67, 0xa7, 0x7c, 0x31}
};

constexpr wchar_t kPipeName[] = L"\\\\.\\pipe\\OmniphonySpatialProviderCanaryV1";
constexpr wchar_t kComBase[] = L"SOFTWARE\\Classes\\CLSID";
constexpr wchar_t kEncoderBase[] = L"SOFTWARE\\Microsoft\\Multimedia\\Audio\\Spatial\\Encoder";
constexpr wchar_t kDisplayName[] = L"Omniphony Spatial Canary (EXPERIMENT)";

std::wstring GuidString(REFGUID guid) {
    wchar_t buffer[64] = {};
    const int count = StringFromGUID2(guid, buffer, static_cast<int>(sizeof(buffer) / sizeof(buffer[0])));
    return count > 1 ? std::wstring(buffer) : std::wstring();
}

std::wstring Win32Message(LONG code) {
    wchar_t buffer[512] = {};
    const DWORD count = FormatMessageW(
        FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
        nullptr,
        static_cast<DWORD>(code),
        0,
        buffer,
        static_cast<DWORD>(sizeof(buffer) / sizeof(buffer[0])),
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

bool IsElevated() {
    HANDLE token = nullptr;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token)) {
        return false;
    }
    TOKEN_ELEVATION elevation = {};
    DWORD bytes = 0;
    const BOOL ok = GetTokenInformation(
        token,
        TokenElevation,
        &elevation,
        sizeof(elevation),
        &bytes);
    CloseHandle(token);
    return ok != FALSE && elevation.TokenIsElevated != 0;
}

std::wstring FullPath(const wchar_t* path) {
    const DWORD required = GetFullPathNameW(path, 0, nullptr, nullptr);
    if (required == 0) {
        return {};
    }
    std::vector<wchar_t> buffer(required + 1, L'\0');
    const DWORD written = GetFullPathNameW(
        path,
        static_cast<DWORD>(buffer.size()),
        buffer.data(),
        nullptr);
    if (written == 0 || written >= buffer.size()) {
        return {};
    }
    return buffer.data();
}

bool WriteString64(const std::wstring& path, const wchar_t* valueName, const std::wstring& value) {
    HKEY key = nullptr;
    const LONG open = RegCreateKeyExW(
        HKEY_LOCAL_MACHINE,
        path.c_str(),
        0,
        nullptr,
        REG_OPTION_NON_VOLATILE,
        KEY_SET_VALUE | KEY_WOW64_64KEY,
        nullptr,
        &key,
        nullptr);
    if (open != ERROR_SUCCESS) {
        std::wcerr << L"registry.create_failed path=HKLM\\" << path
                   << L" error=" << open << L":" << Win32Message(open) << L"\n";
        return false;
    }

    const DWORD bytes = static_cast<DWORD>((value.size() + 1) * sizeof(wchar_t));
    const LONG set = RegSetValueExW(
        key,
        valueName,
        0,
        REG_SZ,
        reinterpret_cast<const BYTE*>(value.c_str()),
        bytes);
    RegCloseKey(key);
    if (set != ERROR_SUCCESS) {
        std::wcerr << L"registry.set_failed path=HKLM\\" << path
                   << L" error=" << set << L":" << Win32Message(set) << L"\n";
        return false;
    }
    return true;
}

std::wstring ReadString64(const std::wstring& path, const wchar_t* valueName) {
    HKEY key = nullptr;
    const LONG open = RegOpenKeyExW(
        HKEY_LOCAL_MACHINE,
        path.c_str(),
        0,
        KEY_QUERY_VALUE | KEY_WOW64_64KEY,
        &key);
    if (open != ERROR_SUCCESS) {
        return {};
    }

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
        RegCloseKey(key);
        return {};
    }

    std::vector<wchar_t> buffer((bytes / sizeof(wchar_t)) + 1, L'\0');
    result = RegGetValueW(
        key,
        nullptr,
        valueName,
        RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ,
        nullptr,
        buffer.data(),
        &bytes);
    RegCloseKey(key);
    return result == ERROR_SUCCESS ? std::wstring(buffer.data()) : std::wstring();
}

bool DeleteTree64(const wchar_t* parentPath, const std::wstring& childName) {
    HKEY parent = nullptr;
    const LONG open = RegOpenKeyExW(
        HKEY_LOCAL_MACHINE,
        parentPath,
        0,
        KEY_WRITE | KEY_WOW64_64KEY,
        &parent);
    if (open == ERROR_FILE_NOT_FOUND) {
        return true;
    }
    if (open != ERROR_SUCCESS) {
        std::wcerr << L"registry.open_parent_failed path=HKLM\\" << parentPath
                   << L" error=" << open << L":" << Win32Message(open) << L"\n";
        return false;
    }

    const LONG result = RegDeleteTreeW(parent, childName.c_str());
    RegCloseKey(parent);
    if (result == ERROR_SUCCESS || result == ERROR_FILE_NOT_FOUND) {
        return true;
    }
    std::wcerr << L"registry.delete_failed parent=HKLM\\" << parentPath
               << L" child=" << childName
               << L" error=" << result << L":" << Win32Message(result) << L"\n";
    return false;
}

bool UnregisterCanary() {
    const std::wstring clsid = GuidString(kProviderClsid);
    const std::wstring format = GuidString(kFormatGuid);
    const bool encoderOk = DeleteTree64(kEncoderBase, format);
    const bool comOk = DeleteTree64(kComBase, clsid);
    if (encoderOk && comOk) {
        std::wcout << L"canary.registration=absent\n";
        return true;
    }
    return false;
}

bool RegisterCanary(const wchar_t* dllArgument) {
    if (!IsElevated()) {
        std::wcerr << L"error=elevation_required\n";
        return false;
    }

    const std::wstring dllPath = FullPath(dllArgument);
    if (dllPath.empty() || GetFileAttributesW(dllPath.c_str()) == INVALID_FILE_ATTRIBUTES) {
        std::wcerr << L"error=provider_dll_not_found path=" << dllArgument << L"\n";
        return false;
    }

    const std::wstring clsid = GuidString(kProviderClsid);
    const std::wstring format = GuidString(kFormatGuid);
    const std::wstring comRoot = std::wstring(kComBase) + L"\\" + clsid;
    const std::wstring inproc = comRoot + L"\\InProcServer32";
    const std::wstring encoder = std::wstring(kEncoderBase) + L"\\" + format;

    bool ok = true;
    ok = ok && WriteString64(comRoot, nullptr, kDisplayName);
    ok = ok && WriteString64(inproc, nullptr, dllPath);
    ok = ok && WriteString64(inproc, L"ThreadingModel", L"Both");
    ok = ok && WriteString64(encoder, nullptr, kDisplayName);
    ok = ok && WriteString64(encoder, L"CLSID", clsid);
    if (!ok) {
        std::wcerr << L"canary.registration=partial rollback=attempted\n";
        UnregisterCanary();
        return false;
    }

    std::wcout << L"canary.registration=present\n";
    std::wcout << L"canary.provider_clsid=" << clsid << L"\n";
    std::wcout << L"canary.format_guid=" << format << L"\n";
    std::wcout << L"canary.dll=" << dllPath << L"\n";
    std::wcout << L"canary.endpoint_state_modified=0\n";
    std::wcout << L"next=run_listen_then_select_canary_in_windows_spatial_sound_ui\n";
    return true;
}

void PrintStatus() {
    const std::wstring clsid = GuidString(kProviderClsid);
    const std::wstring format = GuidString(kFormatGuid);
    const std::wstring comRoot = std::wstring(kComBase) + L"\\" + clsid;
    const std::wstring inproc = comRoot + L"\\InProcServer32";
    const std::wstring encoder = std::wstring(kEncoderBase) + L"\\" + format;

    std::wcout << L"probe=omniphony_spatial_provider_canary\n";
    std::wcout << L"source_truth=experimental_undocumented_provider_surface\n";
    std::wcout << L"provider_clsid=" << clsid << L"\n";
    std::wcout << L"format_guid=" << format << L"\n";
    std::wcout << L"com.display_name=" << ReadString64(comRoot, nullptr) << L"\n";
    std::wcout << L"com.inproc=" << ReadString64(inproc, nullptr) << L"\n";
    std::wcout << L"encoder.display_name=" << ReadString64(encoder, nullptr) << L"\n";
    std::wcout << L"encoder.clsid=" << ReadString64(encoder, L"CLSID") << L"\n";
}

bool ListenOnce() {
    SECURITY_DESCRIPTOR descriptor = {};
    if (!InitializeSecurityDescriptor(&descriptor, SECURITY_DESCRIPTOR_REVISION) ||
        !SetSecurityDescriptorDacl(&descriptor, TRUE, nullptr, FALSE)) {
        std::wcerr << L"error=pipe_security_descriptor_failed code=" << GetLastError() << L"\n";
        return false;
    }

    // The pipe exists only while this explicit diagnostic command is running.
    // A null DACL is acceptable for this one-shot witness because no commands
    // or secrets cross it; the provider sends one requested IID and exits.
    SECURITY_ATTRIBUTES attributes = {};
    attributes.nLength = sizeof(attributes);
    attributes.lpSecurityDescriptor = &descriptor;
    attributes.bInheritHandle = FALSE;

    HANDLE pipe = CreateNamedPipeW(
        kPipeName,
        PIPE_ACCESS_INBOUND,
        PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
        1,
        0,
        4096,
        0,
        &attributes);
    if (pipe == INVALID_HANDLE_VALUE) {
        std::wcerr << L"error=create_pipe_failed code=" << GetLastError() << L"\n";
        return false;
    }

    std::wcout << L"phase1.listener=waiting\n";
    std::wcout << L"action=select_Omniphony_Spatial_Canary_in_Windows_spatial_sound_UI\n";
    std::wcout.flush();

    BOOL connected = ConnectNamedPipe(pipe, nullptr);
    if (connected == FALSE && GetLastError() != ERROR_PIPE_CONNECTED) {
        std::wcerr << L"error=connect_pipe_failed code=" << GetLastError() << L"\n";
        CloseHandle(pipe);
        return false;
    }

    wchar_t message[1024] = {};
    DWORD bytesRead = 0;
    const BOOL read = ReadFile(
        pipe,
        message,
        static_cast<DWORD>(sizeof(message) - sizeof(wchar_t)),
        &bytesRead,
        nullptr);
    CloseHandle(pipe);
    if (read == FALSE || bytesRead == 0 || (bytesRead % sizeof(wchar_t)) != 0) {
        std::wcerr << L"error=read_pipe_failed code=" << GetLastError() << L"\n";
        return false;
    }

    message[bytesRead / sizeof(wchar_t)] = L'\0';
    std::wcout << L"phase1.activation=observed\n";
    std::wcout << message;
    return true;
}

bool Selftest(const wchar_t* dllArgument) {
    const std::wstring dllPath = FullPath(dllArgument);
    if (dllPath.empty()) {
        std::wcerr << L"error=invalid_dll_path\n";
        return false;
    }

    HMODULE module = LoadLibraryW(dllPath.c_str());
    if (module == nullptr) {
        std::wcerr << L"error=load_library_failed code=" << GetLastError() << L"\n";
        return false;
    }

    using GetClassObjectFn = HRESULT(STDAPICALLTYPE*)(REFCLSID, REFIID, LPVOID*);
    const auto getClassObject = reinterpret_cast<GetClassObjectFn>(
        GetProcAddress(module, "DllGetClassObject"));
    if (getClassObject == nullptr) {
        std::wcerr << L"error=missing_DllGetClassObject\n";
        FreeLibrary(module);
        return false;
    }

    IClassFactory* factory = nullptr;
    HRESULT result = getClassObject(
        kProviderClsid,
        IID_IClassFactory,
        reinterpret_cast<void**>(&factory));
    if (FAILED(result) || factory == nullptr) {
        std::wcerr << L"error=class_factory_failed hr=0x" << std::hex
                   << static_cast<unsigned long>(result) << std::dec << L"\n";
        FreeLibrary(module);
        return false;
    }

    void* object = nullptr;
    result = factory->CreateInstance(nullptr, kSelftestIid, &object);
    factory->Release();
    const bool ok = result == E_NOINTERFACE && object == nullptr;
    FreeLibrary(module);

    if (!ok) {
        std::wcerr << L"error=canary_selftest_unexpected_result hr=0x" << std::hex
                   << static_cast<unsigned long>(result) << std::dec << L"\n";
        return false;
    }

    std::wcout << L"canary.selftest=pass\n";
    std::wcout << L"canary.sentinel_iid=" << GuidString(kSelftestIid) << L"\n";
    std::wcout << L"canary.behavior=records_requested_iid_then_returns_E_NOINTERFACE\n";
    return true;
}

void Usage() {
    std::wcout
        << L"Omniphony Spatial Provider Canary\n"
        << L"\n"
        << L"  OmniphonySpatialProviderCanaryCtl.exe status\n"
        << L"  OmniphonySpatialProviderCanaryCtl.exe register <OmniphonySpatialProviderCanary.dll>\n"
        << L"  OmniphonySpatialProviderCanaryCtl.exe listen\n"
        << L"  OmniphonySpatialProviderCanaryCtl.exe selftest <OmniphonySpatialProviderCanary.dll>\n"
        << L"  OmniphonySpatialProviderCanaryCtl.exe unregister\n"
        << L"\n"
        << L"Registration is experimental and writes only the observed 64-bit Spatial\\Encoder\n"
        << L"and COM CLSID keys. It does not select a spatial format or modify MMDevices state.\n";
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc < 2) {
        Usage();
        return 2;
    }

    const std::wstring command = argv[1];
    if (command == L"status") {
        PrintStatus();
        return 0;
    }
    if (command == L"register") {
        if (argc != 3) {
            Usage();
            return 2;
        }
        return RegisterCanary(argv[2]) ? 0 : 1;
    }
    if (command == L"unregister") {
        if (!IsElevated()) {
            std::wcerr << L"error=elevation_required\n";
            return 1;
        }
        return UnregisterCanary() ? 0 : 1;
    }
    if (command == L"listen") {
        return ListenOnce() ? 0 : 1;
    }
    if (command == L"selftest") {
        if (argc != 3) {
            Usage();
            return 2;
        }
        return Selftest(argv[2]) ? 0 : 1;
    }

    Usage();
    return 2;
}
