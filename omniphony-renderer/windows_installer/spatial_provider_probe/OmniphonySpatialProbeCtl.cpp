#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <objbase.h>

#include <iomanip>
#include <iostream>
#include <sstream>
#include <string>
#include <utility>

namespace {

constexpr int kExitUsage = 2;
constexpr int kExitNotRegistered = 3;
constexpr int kExitAccess = 4;
constexpr int kExitVerify = 5;

constexpr wchar_t kDisplayName[] = L"Omniphony";
constexpr wchar_t kFormatGuid[] = L"{4BD75423-A66C-4586-B782-1FCBBDF2AE74}";
constexpr wchar_t kClsidText[] = L"{F3CDF827-20C4-405E-A430-8F739343FC89}";
constexpr GUID kProbeClsid = {
    0xf3cdf827, 0x20c4, 0x405e, {0xa4, 0x30, 0x8f, 0x73, 0x93, 0x43, 0xfc, 0x89}};

constexpr wchar_t kEncoderBase[] = L"SOFTWARE\\Microsoft\\Multimedia\\Audio\\Spatial\\Encoder";
constexpr wchar_t kComBase[] = L"SOFTWARE\\Classes\\CLSID";

std::wstring Join(const wchar_t* left, const wchar_t* right) {
    std::wstring value(left);
    value += L"\\";
    value += right;
    return value;
}

std::wstring Win32Text(DWORD error) {
    wchar_t* buffer = nullptr;
    const DWORD flags = FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM |
                        FORMAT_MESSAGE_IGNORE_INSERTS;
    const DWORD count = FormatMessageW(
        flags,
        nullptr,
        error,
        MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT),
        reinterpret_cast<wchar_t*>(&buffer),
        0,
        nullptr);

    std::wostringstream out;
    out << error << L" (0x" << std::uppercase << std::hex << std::setw(8)
        << std::setfill(L'0') << error << L")";
    if (count && buffer) {
        std::wstring message(buffer, count);
        while (!message.empty() && (message.back() == L'\r' || message.back() == L'\n')) {
            message.pop_back();
        }
        out << L" " << message;
    }
    if (buffer) {
        LocalFree(buffer);
    }
    return out.str();
}

bool IsElevated() {
    HANDLE token = nullptr;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token)) {
        return false;
    }
    TOKEN_ELEVATION elevation{};
    DWORD size = sizeof(elevation);
    const bool ok = GetTokenInformation(
        token, TokenElevation, &elevation, sizeof(elevation), &size) != FALSE;
    CloseHandle(token);
    return ok && elevation.TokenIsElevated != 0;
}

bool FileExists(const std::wstring& path) {
    const DWORD attributes = GetFileAttributesW(path.c_str());
    return attributes != INVALID_FILE_ATTRIBUTES &&
           (attributes & FILE_ATTRIBUTE_DIRECTORY) == 0;
}

bool AbsolutePath(const wchar_t* input, std::wstring& output) {
    const DWORD needed = GetFullPathNameW(input, 0, nullptr, nullptr);
    if (needed == 0) {
        return false;
    }
    std::wstring buffer(needed, L'\0');
    const DWORD written = GetFullPathNameW(input, needed, buffer.data(), nullptr);
    if (written == 0 || written >= needed) {
        return false;
    }
    buffer.resize(written);
    output = std::move(buffer);
    return true;
}

LONG SetString(HKEY key, const wchar_t* name, const std::wstring& value) {
    const DWORD bytes = static_cast<DWORD>((value.size() + 1) * sizeof(wchar_t));
    return RegSetValueExW(
        key,
        name,
        0,
        REG_SZ,
        reinterpret_cast<const BYTE*>(value.c_str()),
        bytes);
}

bool ReadString(HKEY root, const std::wstring& path, const wchar_t* name, std::wstring& value) {
    HKEY key = nullptr;
    LONG result = RegOpenKeyExW(root, path.c_str(), 0, KEY_READ, &key);
    if (result != ERROR_SUCCESS) {
        return false;
    }

    DWORD type = 0;
    DWORD bytes = 0;
    result = RegQueryValueExW(key, name, nullptr, &type, nullptr, &bytes);
    if (result != ERROR_SUCCESS || (type != REG_SZ && type != REG_EXPAND_SZ) || bytes < sizeof(wchar_t)) {
        RegCloseKey(key);
        return false;
    }

    std::wstring buffer(bytes / sizeof(wchar_t), L'\0');
    result = RegQueryValueExW(
        key,
        name,
        nullptr,
        &type,
        reinterpret_cast<BYTE*>(buffer.data()),
        &bytes);
    RegCloseKey(key);
    if (result != ERROR_SUCCESS) {
        return false;
    }

    if (!buffer.empty() && buffer.back() == L'\0') {
        buffer.pop_back();
    }
    value = std::move(buffer);
    return true;
}

bool KeyExists(HKEY root, const std::wstring& path) {
    HKEY key = nullptr;
    const LONG result = RegOpenKeyExW(root, path.c_str(), 0, KEY_READ, &key);
    if (result == ERROR_SUCCESS) {
        RegCloseKey(key);
        return true;
    }
    return false;
}

void PrintContract() {
    std::wcout << L"FORMAT_GUID\t" << kFormatGuid << L'\n';
    std::wcout << L"COM_CLSID\t" << kClsidText << L'\n';
    std::wcout << L"ENCODER_BASE\tHKLM\\" << kEncoderBase << L'\n';
    std::wcout << L"COM_BASE\tHKLM\\" << kComBase << L'\n';
    std::wcout << L"NO_AUDIO_PROCESSING\t1\n";
    std::wcout << L"NO_MMDEVICES_WRITES\t1\n";
    std::wcout << L"NO_DEFAULT_ENDPOINT_CHANGE\t1\n";
}

int ListProviders() {
    HKEY root = nullptr;
    const LONG open = RegOpenKeyExW(HKEY_LOCAL_MACHINE, kEncoderBase, 0, KEY_READ, &root);
    if (open == ERROR_FILE_NOT_FOUND) {
        std::wcout << L"SPATIAL_ENCODER_BASE_ABSENT\n";
        return 0;
    }
    if (open != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\topen Spatial\\Encoder\t" << Win32Text(open) << L'\n';
        return kExitAccess;
    }

    DWORD index = 0;
    bool any = false;
    for (;;) {
        wchar_t name[256] = {};
        DWORD chars = static_cast<DWORD>(sizeof(name) / sizeof(name[0]));
        FILETIME time{};
        const LONG result = RegEnumKeyExW(root, index++, name, &chars, nullptr, nullptr, nullptr, &time);
        if (result == ERROR_NO_MORE_ITEMS) {
            break;
        }
        if (result != ERROR_SUCCESS) {
            RegCloseKey(root);
            std::wcerr << L"ERROR\tenumerate Spatial\\Encoder\t" << Win32Text(result) << L'\n';
            return kExitAccess;
        }

        any = true;
        const std::wstring path = Join(kEncoderBase, name);
        std::wstring display;
        std::wstring clsid;
        ReadString(HKEY_LOCAL_MACHINE, path, nullptr, display);
        ReadString(HKEY_LOCAL_MACHINE, path, L"CLSID", clsid);
        std::wcout << L"SPATIAL_ENCODER\t" << name
                   << L"\tNAME=" << (display.empty() ? L"<none>" : display)
                   << L"\tCLSID=" << (clsid.empty() ? L"<none>" : clsid) << L'\n';
    }
    RegCloseKey(root);
    if (!any) {
        std::wcout << L"SPATIAL_ENCODER_NONE\n";
    }
    return 0;
}

int Status() {
    const std::wstring encoderPath = Join(kEncoderBase, kFormatGuid);
    const std::wstring classPath = Join(kComBase, kClsidText);
    const std::wstring inprocPath = classPath + L"\\InProcServer32";

    std::wstring display;
    std::wstring clsid;
    std::wstring server;
    const bool encoder = KeyExists(HKEY_LOCAL_MACHINE, encoderPath);
    const bool com = KeyExists(HKEY_LOCAL_MACHINE, inprocPath);
    if (encoder) {
        ReadString(HKEY_LOCAL_MACHINE, encoderPath, nullptr, display);
        ReadString(HKEY_LOCAL_MACHINE, encoderPath, L"CLSID", clsid);
    }
    if (com) {
        ReadString(HKEY_LOCAL_MACHINE, inprocPath, nullptr, server);
    }

    std::wcout << L"SPATIAL_PROBE_STATUS\tENCODER=" << (encoder ? 1 : 0)
               << L"\tCOM=" << (com ? 1 : 0) << L'\n';
    std::wcout << L"FORMAT_GUID\t" << kFormatGuid << L'\n';
    std::wcout << L"COM_CLSID\t" << kClsidText << L'\n';
    if (encoder) {
        std::wcout << L"ENCODER_NAME\t" << (display.empty() ? L"<none>" : display) << L'\n';
        std::wcout << L"ENCODER_CLSID\t" << (clsid.empty() ? L"<none>" : clsid) << L'\n';
    }
    if (com) {
        std::wcout << L"COM_SERVER\t" << (server.empty() ? L"<none>" : server) << L'\n';
    }
    return (encoder && com) ? 0 : kExitNotRegistered;
}

LONG DeleteOwnedKey(const std::wstring& path) {
    const LONG result = RegDeleteTreeW(HKEY_LOCAL_MACHINE, path.c_str());
    if (result == ERROR_FILE_NOT_FOUND || result == ERROR_PATH_NOT_FOUND) {
        return ERROR_SUCCESS;
    }
    return result;
}

int UnregisterOwnedKeys() {
    if (!IsElevated()) {
        std::wcerr << L"ERROR\tspatial-unregister requires an elevated Administrator terminal\n";
        return kExitAccess;
    }

    const std::wstring encoderPath = Join(kEncoderBase, kFormatGuid);
    const std::wstring classPath = Join(kComBase, kClsidText);

    const LONG encoder = DeleteOwnedKey(encoderPath);
    const LONG com = DeleteOwnedKey(classPath);
    if (encoder != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tdelete Omniphony Spatial\\Encoder key\t" << Win32Text(encoder) << L'\n';
        return kExitAccess;
    }
    if (com != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tdelete Omniphony COM key\t" << Win32Text(com) << L'\n';
        return kExitAccess;
    }

    std::wcout << L"SPATIAL_PROBE_UNREGISTERED\tFORMAT_GUID=" << kFormatGuid
               << L"\tCLSID=" << kClsidText << L'\n';
    return 0;
}

int RegisterOwnedKeys(const wchar_t* dllArgument) {
    if (!IsElevated()) {
        std::wcerr << L"ERROR\tspatial-register requires an elevated Administrator terminal\n";
        return kExitAccess;
    }

    std::wstring dllPath;
    if (!AbsolutePath(dllArgument, dllPath) || !FileExists(dllPath)) {
        std::wcerr << L"ERROR\tprobe DLL not found\t" << dllArgument << L'\n';
        return kExitUsage;
    }

    const std::wstring classPath = Join(kComBase, kClsidText);
    const std::wstring inprocPath = classPath + L"\\InProcServer32";
    const std::wstring encoderPath = Join(kEncoderBase, kFormatGuid);

    HKEY classKey = nullptr;
    LONG result = RegCreateKeyExW(
        HKEY_LOCAL_MACHINE, classPath.c_str(), 0, nullptr, REG_OPTION_NON_VOLATILE,
        KEY_WRITE, nullptr, &classKey, nullptr);
    if (result != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tcreate Omniphony COM class\t" << Win32Text(result) << L'\n';
        return kExitAccess;
    }
    result = SetString(classKey, nullptr, L"Omniphony Spatial Provider Probe");
    RegCloseKey(classKey);
    if (result != ERROR_SUCCESS) {
        DeleteOwnedKey(classPath);
        std::wcerr << L"ERROR\tname Omniphony COM class\t" << Win32Text(result) << L'\n';
        return kExitAccess;
    }

    HKEY inprocKey = nullptr;
    result = RegCreateKeyExW(
        HKEY_LOCAL_MACHINE, inprocPath.c_str(), 0, nullptr, REG_OPTION_NON_VOLATILE,
        KEY_WRITE, nullptr, &inprocKey, nullptr);
    if (result == ERROR_SUCCESS) {
        result = SetString(inprocKey, nullptr, dllPath);
    }
    if (result == ERROR_SUCCESS) {
        result = SetString(inprocKey, L"ThreadingModel", L"Both");
    }
    if (inprocKey) {
        RegCloseKey(inprocKey);
    }
    if (result != ERROR_SUCCESS) {
        DeleteOwnedKey(classPath);
        std::wcerr << L"ERROR\tregister Omniphony InProcServer32\t" << Win32Text(result) << L'\n';
        return kExitAccess;
    }

    HKEY encoderKey = nullptr;
    result = RegCreateKeyExW(
        HKEY_LOCAL_MACHINE, encoderPath.c_str(), 0, nullptr, REG_OPTION_NON_VOLATILE,
        KEY_WRITE, nullptr, &encoderKey, nullptr);
    if (result == ERROR_SUCCESS) {
        result = SetString(encoderKey, nullptr, kDisplayName);
    }
    if (result == ERROR_SUCCESS) {
        result = SetString(encoderKey, L"CLSID", kClsidText);
    }
    if (encoderKey) {
        RegCloseKey(encoderKey);
    }
    if (result != ERROR_SUCCESS) {
        DeleteOwnedKey(encoderPath);
        DeleteOwnedKey(classPath);
        std::wcerr << L"ERROR\tregister Omniphony Spatial\\Encoder format\t"
                   << Win32Text(result) << L'\n';
        return kExitAccess;
    }

    const HRESULT init = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(init) && init != RPC_E_CHANGED_MODE) {
        DeleteOwnedKey(encoderPath);
        DeleteOwnedKey(classPath);
        std::wcerr << L"ERROR\tCoInitializeEx failed; registration rolled back\t0x"
                   << std::uppercase << std::hex << static_cast<unsigned long>(init) << L'\n';
        return kExitVerify;
    }

    IUnknown* probe = nullptr;
    const HRESULT activate = CoCreateInstance(
        kProbeClsid, nullptr, CLSCTX_INPROC_SERVER, IID_IUnknown,
        reinterpret_cast<void**>(&probe));
    if (probe) {
        probe->Release();
    }
    if (SUCCEEDED(init)) {
        CoUninitialize();
    }
    if (FAILED(activate)) {
        DeleteOwnedKey(encoderPath);
        DeleteOwnedKey(classPath);
        std::wcerr << L"ERROR\tprobe COM activation failed; registration rolled back\t0x"
                   << std::uppercase << std::hex << static_cast<unsigned long>(activate) << L'\n';
        return kExitVerify;
    }

    std::wcout << L"SPATIAL_PROBE_REGISTERED\tFORMAT_GUID=" << kFormatGuid
               << L"\tCLSID=" << kClsidText << L"\tDLL=" << dllPath << L'\n';
    std::wcout << L"COM_ACTIVATION_OK\tIUnknown\n";
    std::wcout << L"NEXT\tReopen Settings > System > Sound > your output > Spatial sound and check for Omniphony.\n";
    std::wcout << L"BOUNDARY\tMenu appearance proves enumeration only; this probe does not implement a spatial renderer.\n";
    return 0;
}

int Diagnose() {
    const int status = Status();
    const int listed = ListProviders();
    if (listed != 0) {
        return listed;
    }
    if (status != 0) {
        std::wcerr << L"DIAGNOSIS\tOmniphony probe is not fully registered.\n";
        return status;
    }

    const HRESULT init = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(init) && init != RPC_E_CHANGED_MODE) {
        std::wcerr << L"ERROR\tCoInitializeEx\t0x" << std::uppercase << std::hex
                   << static_cast<unsigned long>(init) << L'\n';
        return kExitVerify;
    }

    IUnknown* probe = nullptr;
    const HRESULT activate = CoCreateInstance(
        kProbeClsid, nullptr, CLSCTX_INPROC_SERVER, IID_IUnknown,
        reinterpret_cast<void**>(&probe));
    if (probe) {
        probe->Release();
    }
    if (SUCCEEDED(init)) {
        CoUninitialize();
    }
    if (FAILED(activate)) {
        std::wcerr << L"ERROR\tCoCreateInstance probe\t0x" << std::uppercase << std::hex
                   << static_cast<unsigned long>(activate) << L'\n';
        return kExitVerify;
    }

    std::wcout << L"COM_ACTIVATION_OK\tIUnknown\n";
    std::wcout << L"DIAGNOSIS\tRegistry state and inert COM activation are internally consistent.\n";
    std::wcout << L"BOUNDARY\tWindows Settings enumeration and spatial-renderer activation still require a real machine test.\n";
    return 0;
}

void Usage() {
    std::wcerr
        << L"usage: OmniphonySpatialProbeCtl <contract|list|status|register|diagnose|unregister> [probe-dll]\n"
        << L"  contract                 print stable GUIDs and safety boundaries\n"
        << L"  list                     list current HKLM Spatial\\Encoder entries (read-only)\n"
        << L"  status                   inspect only Omniphony-owned registration keys (read-only)\n"
        << L"  register <probe-dll>     register inert Omniphony provider probe (Administrator)\n"
        << L"  diagnose                 verify registry plus IUnknown COM activation (read-only)\n"
        << L"  unregister               remove only Omniphony probe keys (Administrator)\n";
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc < 2) {
        Usage();
        return kExitUsage;
    }

    const std::wstring command = argv[1];
    if (command == L"contract") {
        PrintContract();
        return 0;
    }
    if (command == L"list") {
        return ListProviders();
    }
    if (command == L"status") {
        return Status();
    }
    if (command == L"register") {
        if (argc != 3 || !argv[2] || !*argv[2]) {
            Usage();
            return kExitUsage;
        }
        return RegisterOwnedKeys(argv[2]);
    }
    if (command == L"diagnose") {
        return Diagnose();
    }
    if (command == L"unregister") {
        return UnregisterOwnedKeys();
    }

    Usage();
    return kExitUsage;
}
