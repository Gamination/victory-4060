"""Install only the SDK 2.4.0 PC upscaler runtime, outside the source tree.
No DLLs are executed. Download is explicit, and both extracted DLLs are pinned.
"""
import argparse
import hashlib
import os
from pathlib import Path
import shutil
import tempfile
import urllib.request
import zipfile

URL = 'https://raw.githubusercontent.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK/v2.4.0/Kits/FidelityFX/signedbin/'
FILES = {
    'amd_fidelityfx_loader_dx12.dll': 'e2191d3c0828be45eb8affd4e3ec0378f2293adc345b772a7170edcf3b021ecb',
    'amd_fidelityfx_upscaler_dx12.dll': 'aad4fccccb29084a655f9a4f9f613bd22c8c03fb4a0f8840148cc6543b4ccace',
}

def digest(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

def install(archive, dest):
    dest = dest.resolve()
    repo = Path(__file__).resolve().parents[1]
    if dest == repo or repo in dest.parents:
        raise ValueError('SDK directory must be outside the source tree')
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists():
        if all((dest/n).is_file() and digest(dest/n) == h for n,h in FILES.items()):
            return dest
        raise ValueError(f'Destination exists with other contents: {dest}')
    with tempfile.TemporaryDirectory(dir=dest.parent) as tmp:
        ready = Path(tmp) / 'runtime'
        ready.mkdir()
        if archive is None:
            for name in FILES:
                print('Downloading', name, flush=True)
                with urllib.request.urlopen(URL + name, timeout=90) as src, (ready / name).open('wb') as out:
                    shutil.copyfileobj(src, out)
        else:
            with zipfile.ZipFile(archive) as z:
                for name in FILES:
                    suffix = '/Kits/FidelityFX/signedbin/' + name
                    matches = [n for n in z.namelist() if n.endswith(suffix)]
                    if len(matches) != 1:
                        raise ValueError(f'Expected one signed SDK member for {name}')
                    with z.open(matches[0]) as src, (ready / name).open('wb') as out:
                        shutil.copyfileobj(src, out)
        for name, expected in FILES.items():
            if digest(ready / name) != expected:
                raise ValueError(f'SHA-256 mismatch: {name}; no files installed')
        ready.rename(dest)
    return dest

def main():
    p = argparse.ArgumentParser(description=__doc__)
    source = p.add_mutually_exclusive_group(required=True)
    source.add_argument('--zip', type=Path)
    source.add_argument('--download', action='store_true')
    default = Path(os.environ.get('LOCALAPPDATA', str(Path.home()))) / 'VIctory' / 'sdk' / 'ffx-2.4.0'
    p.add_argument('--dest', type=Path, default=default)
    args = p.parse_args()
    dest = install(args.zip, args.dest)
    print('Installed and SHA-256 verified:', dest)
    print("$env:VICTORY_PC_FFX_DIR = '" + str(dest).replace("'", "''") + "'")

if __name__ == '__main__':
    main()
