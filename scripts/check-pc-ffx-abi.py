"""Compile x64 ABI assertions against the supplied SDK headers, without running a DLL.
Requires a C11 compiler (clang or cc) and the original SDK ZIP.
"""
import argparse
from pathlib import Path
import subprocess
import tempfile
import zipfile

CHECK = r'''
#include <stddef.h>
#define __declspec(x)
#include "upscalers/include/ffx_upscale.h"
#define SIZE(t,n) _Static_assert(sizeof(struct t)==(n), #t " size")
#define OFF(t,f,n) _Static_assert(offsetof(struct t,f)==(n), #t "." #f)
#define EQ(a,b) _Static_assert((a)==(b), #a)
SIZE(ffxApiHeader,16); SIZE(ffxQueryDescGetVersions,56);
SIZE(ffxCreateContextDescUpscale,48); SIZE(ffxCreateContextDescUpscaleVersion,24);
SIZE(ffxOverrideVersion,24); SIZE(ffxQueryGetProviderVersion,32);
SIZE(ffxQueryDescUpscaleGetResourceRequirements,32);
SIZE(ffxQueryDescUpscaleGetGPUMemoryUsage,24); SIZE(FfxApiEffectMemoryUsage,16);
SIZE(FfxApiResourceDescription,32); SIZE(FfxApiResource,48);
OFF(FfxApiResource,state,40);
SIZE(ffxDispatchDescUpscale,0x1b0);
OFF(ffxDispatchDescUpscale,commandList,0x10);
OFF(ffxDispatchDescUpscale,color,0x18);
OFF(ffxDispatchDescUpscale,depth,0x48);
OFF(ffxDispatchDescUpscale,motionVectors,0x78);
OFF(ffxDispatchDescUpscale,exposure,0xa8);
OFF(ffxDispatchDescUpscale,reactive,0xd8);
OFF(ffxDispatchDescUpscale,transparencyAndComposition,0x108);
OFF(ffxDispatchDescUpscale,output,0x138);
OFF(ffxDispatchDescUpscale,jitterOffset,0x168);
OFF(ffxDispatchDescUpscale,motionVectorScale,0x170);
OFF(ffxDispatchDescUpscale,renderSize,0x178);
OFF(ffxDispatchDescUpscale,upscaleSize,0x180);
OFF(ffxDispatchDescUpscale,enableSharpening,0x188);
OFF(ffxDispatchDescUpscale,sharpness,0x18c);
OFF(ffxDispatchDescUpscale,frameTimeDelta,0x190);
OFF(ffxDispatchDescUpscale,preExposure,0x194);
OFF(ffxDispatchDescUpscale,reset,0x198);
OFF(ffxDispatchDescUpscale,cameraNear,0x19c);
OFF(ffxDispatchDescUpscale,cameraFar,0x1a0);
OFF(ffxDispatchDescUpscale,cameraFovAngleVertical,0x1a4);
OFF(ffxDispatchDescUpscale,viewSpaceToMetersFactor,0x1a8);
OFF(ffxDispatchDescUpscale,flags,0x1ac);
EQ(FFX_API_CREATE_CONTEXT_DESC_TYPE_UPSCALE,0x10000);
EQ(FFX_API_DISPATCH_DESC_TYPE_UPSCALE,0x10001);
EQ(FFX_API_QUERY_DESC_TYPE_UPSCALE_GPU_MEMORY_USAGE,0x10008);
EQ(FFX_API_QUERY_DESC_TYPE_UPSCALE_GET_RESOURCE_REQUIREMENTS,0x1000a);
EQ(FFX_API_CREATE_CONTEXT_DESC_TYPE_UPSCALE_VERSION,0x1000b);
EQ(FFX_UPSCALER_VERSION,(4<<22)|(1<<12)|2);
EQ(FFX_UPSCALE_ENABLE_AUTO_EXPOSURE,32);
EQ(FFX_API_SURFACE_FORMAT_R16G16B16A16_FLOAT,4);
EQ(FFX_API_SURFACE_FORMAT_R32_FLOAT,28);
EQ(FFX_API_SURFACE_FORMAT_R16G16_FLOAT,18);
EQ(FFX_API_SURFACE_FORMAT_R8G8B8A8_UNORM,10);
EQ(FFX_API_RESOURCE_TYPE_TEXTURE2D,2);
EQ(FFX_API_RESOURCE_USAGE_UAV,2);
EQ(FFX_API_RESOURCE_STATE_COMPUTE_READ,4);
EQ(FFX_API_RESOURCE_STATE_UNORDERED_ACCESS,2);
'''

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('archive',type=Path)
    p.add_argument('--cc',default='clang')
    a=p.parse_args()
    with tempfile.TemporaryDirectory() as temp, zipfile.ZipFile(a.archive) as z:
        root=Path(temp)
        for path in ('api/include/ffx_api.h','api/include/ffx_api_types.h','upscalers/include/ffx_upscale.h'):
            matches=[n for n in z.namelist() if n.endswith('/Kits/FidelityFX/'+path)]
            if len(matches)!=1: raise ValueError(f'Expected one SDK header: {path}')
            dest=root/path
            dest.parent.mkdir(parents=True,exist_ok=True)
            dest.write_bytes(z.read(matches[0]))
        c=root/'check.c'; c.write_text(CHECK)
        subprocess.run([a.cc,'-std=c11','-fsyntax-only',str(c)],check=True)
    print('SDK 2.4 x64 header ABI assertions passed (C compilation only; Rust and GPU runtime not tested).')

if __name__=='__main__': main()
