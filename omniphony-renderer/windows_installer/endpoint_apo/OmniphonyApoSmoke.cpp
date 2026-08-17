#include <windows.h>
#include <audioenginebaseapo.h>
#include <iostream>
#include <wrl/client.h>

using Microsoft::WRL::ComPtr;

namespace {
constexpr GUID kOmniphonyApoClsid = {
    0xa9333bfe, 0x39c1, 0x40fd, {0xb4, 0xb0, 0xec, 0xc5, 0x91, 0x41, 0x0b, 0x47}};
}

int wmain() {
    const HRESULT init = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(init)) {
        std::wcerr << L"COM_INIT_FAILED\t0x" << std::hex << init << std::endl;
        return 2;
    }

    int result = 0;
    {
        std::wcout << L"SMOKE_STAGE\tCOCREATE_BEGIN" << std::endl;
        ComPtr<IAudioProcessingObject> apo;
        HRESULT hr = CoCreateInstance(kOmniphonyApoClsid, nullptr, CLSCTX_INPROC_SERVER,
                                      IID_PPV_ARGS(apo.ReleaseAndGetAddressOf()));
        if (FAILED(hr)) {
            std::wcerr << L"APO_ACTIVATION_FAILED\t0x" << std::hex << hr << std::endl;
            result = 3;
        } else {
            std::wcout << L"SMOKE_STAGE\tCOCREATE_OK" << std::endl;

            ComPtr<IAudioProcessingObjectRT> rt;
            ComPtr<IAudioProcessingObjectConfiguration> configuration;
            ComPtr<IAudioSystemEffects> effects;

            std::wcout << L"SMOKE_STAGE\tQI_RT_BEGIN" << std::endl;
            const HRESULT rtHr = apo.As(&rt);
            std::wcout << L"SMOKE_STAGE\tQI_RT_END\t0x" << std::hex << rtHr << std::endl;

            std::wcout << L"SMOKE_STAGE\tQI_CFG_BEGIN" << std::endl;
            const HRESULT cfgHr = apo.As(&configuration);
            std::wcout << L"SMOKE_STAGE\tQI_CFG_END\t0x" << std::hex << cfgHr << std::endl;

            std::wcout << L"SMOKE_STAGE\tQI_FX_BEGIN" << std::endl;
            const HRESULT fxHr = apo.As(&effects);
            std::wcout << L"SMOKE_STAGE\tQI_FX_END\t0x" << std::hex << fxHr << std::endl;

            if (FAILED(rtHr) || FAILED(cfgHr) || FAILED(fxHr)) {
                std::wcerr << L"APO_INTERFACE_FAILED\tRT=0x" << std::hex << rtHr
                           << L"\tCFG=0x" << cfgHr << L"\tFX=0x" << fxHr << std::endl;
                result = 4;
            } else {
                HNSTIME latency = -1;
                std::wcout << L"SMOKE_STAGE\tLATENCY_BEGIN" << std::endl;
                hr = apo->GetLatency(&latency);
                std::wcout << L"SMOKE_STAGE\tLATENCY_END\t0x" << std::hex << hr
                           << L"\t" << std::dec << latency << std::endl;
                if (FAILED(hr) || latency != 0) {
                    std::wcerr << L"APO_LATENCY_FAILED\tHR=0x" << std::hex << hr
                               << L"\tLATENCY=" << std::dec << latency << std::endl;
                    result = 5;
                } else {
                    std::wcout << L"APO_COM_OK\tCLSID={A9333BFE-39C1-40FD-B4B0-ECC591410B47}\tLATENCY_HNS=0" << std::endl;
                }
            }
        }
    }

    CoUninitialize();
    return result;
}
