import subprocess, sys, os

release = False
gnu = False

x86 = "i686"
x64 = "x86_64";

if len(sys.argv) >= 2:
    release = (sys.argv[1] == "True") | (sys.argv[1] == "true")
if len(sys.argv) >= 3:
    gnu = (sys.argv[2] == "True") | (sys.argv[2] == "true")

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
        dst = f'{path}{os.path.sep}{assembly}_{x86}.{trail}'
        try:
            if os.path.exists(dst):
                os.remove(dst)
        except OSError as e:
            print(f'\033[0;31mError\033[0m: {e}')

        os.rename(f'{path}{os.path.sep}target{os.path.sep}{target}{os.path.sep}{profile_dir}{os.path.sep}{assembly}.{trail}', dst)
