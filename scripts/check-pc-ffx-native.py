"""Offline check of the first native bridge's descriptor eligibility model.
Optional --log reads *observed* guest metadata; native handles/lifetimes and
GPU execution cannot be validated by this model. No Windows DLLs are loaded.
"""
import argparse
import re
import math
from pathlib import Path


def fits(a, b):
    return all(0 < x <= y <= 8192 for x, y in zip(a, b))


def eligible(c, r, o, flags=0, optional=False, fmt=(10,19,34,10), alias=False):
    cf, mr, mo = c
    o = mo if o == (0, 0) else o
    return (cf & ~0x7f == 0 and cf & 0x20 and fits(mr, mo)
            and fits(r, mr) and fits(o, mo) and fits(r, o)
            and flags == 0 and not optional and not alias
            and fmt == (10, 19, 34, 10))


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--log', type=Path)
    args = ap.parse_args()
    c = (0x69, (2560,1440), (3840,2160))
    assert eligible(c,(1280,720),(1920,1080))
    assert eligible(c,(1920,1080),(3840,2160))
    assert eligible(c,(800,600),(800,600))
    assert eligible(c,(1920,1080),(0,0))
    assert not eligible(c,(2561,1440),(3840,2160))
    assert not eligible(c,(0,720),(1920,1080))
    assert not eligible(c,(1280,720),(1920,1080),flags=1)
    assert not eligible(c,(1280,720),(1920,1080),optional=True)
    assert not eligible(c,(1280,720),(1920,1080),alias=True)
    assert not eligible(c,(1280,720),(1920,1080),fmt=(10,40,34,10))
    print('10 native descriptor model checks passed; not Rust or GPU tests.')
    if args.log:
        raw = args.log.read_bytes()
        text = raw.decode('utf-16' if raw.startswith((b'\xff\xfe',b'\xfe\xff')) else 'utf-8', errors='replace')
        # Log lines are PowerShell-wrapped, so use whitespace-aware patterns.
        text = re.sub(r'\s+', ' ', text)
        create = re.search(r'guest upscale create layout flags=(\d+) render=\[(\d+), (\d+)\] output=\[(\d+), (\d+)\]',text)
        if not create:
            raise SystemExit('no guest create layout found')
        f,rw,rh,ow,oh=map(int,create.groups());c=(f,(rw,rh),(ow,oh))
        pattern=r'guest upscale dispatch layout candidate (.*?)(?=\d{4}-\d{2}-\d{2}T|$)'
        count=valid=0
        for match in re.finditer(pattern,text):
            x=match.group(1); d=re.search(r'render=\[(\d+), (\d+)\] output=\[(\d+), (\d+)\]',x)
            if not d: continue
            r1,r2,o1,o2=map(int,d.groups());flags=re.search(r'flags=(\d+)\s*$',x)
            optional=any(not re.search(r'\("'+name+r'", 0,',x) for name in ('exposure','reactive','transparency'))
            params=re.search(r'delta_ms=([\d.eE+-]+) pre_exposure=([\d.eE+-]+) reset=[01] near=([\d.eE+-]+) far=([\d.eE+-]+) fov=([\d.eE+-]+) meters=([\d.eE+-]+)',x)
            finite=False
            if params:
                dt,exp,near,far,fov,meters=map(float,params.groups())
                finite=all(map(math.isfinite,(dt,exp,near,far,fov,meters))) and dt>=0 and exp>0 and near>0 and far>0 and 0<fov<math.pi and meters>=0
            count+=1
            valid+=bool(flags and finite and eligible(c,(r1,r2),(o1,o2),int(flags.group(1)),optional))
        print(f'Observed create limits={c}; {valid}/{count} sampled dispatches pass scalar/dimension/mask model (not native-resource eligibility).')

if __name__=='__main__':
    main()
