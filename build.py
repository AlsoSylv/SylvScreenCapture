import subprocess, sys, os

release = False
gnu = False

x86 = "i686"
x64 = "x86_64";

match (len(sys.argv)):
    case 2: 
        bool(sys.argv[1])
    case 3: 
        bool(sys.argv[1])
        bool(sys.argv[2])

profile = "release" if release else "dev"
profile_dir = "release" if release else "debug"
gnu = "gnu" if gnu else "msvc"

targets = [(f'{x86}-pc-windows-{gnu}', True), (f'{x64}-pc-windows-{gnu}', False)]
build = ['cargo', 'build', '-p', '', f'--profile={profile}', '']

assemblys = [('load_library_getter', True), ('screen_recorder', False)]

for (assembly, exe) in assemblys:
    build[3] = assembly
    for (target, x86) in targets:
        build[5] = f'--target={target}'
        result = subprocess.run(build)
        if result.returncode != 0:
            exit(1)
        trail = 'exe' if exe else 'dll'
        x86 = "32" if x86 else "64"
        path = os.path.dirname(os.path.abspath(__file__))
        dst = f'{path}\\{assembly}_{x86}.{trail}'
        try:
            os.remove(dst)
        except:
            {}
        os.rename(f'{path}\\target\\{target}\\{profile_dir}\\{assembly}.{trail}', dst)
