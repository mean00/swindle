#!/usr/bin/env python3
import os
import shutil
import subprocess
import sys
from pathlib import Path

# Try to import rich for modern UI, fallback to standard print if not available
# Check if we are in a CI environment or non-interactive terminal
is_ci = (
    os.environ.get("CI") in ("1", "true", "True") 
    or "JENKINS_URL" in os.environ 
    or "GITHUB_ACTIONS" in os.environ 
    or not sys.stdout.isatty()
)

try:
    from rich.console import Console
    from rich.progress import Progress, SpinnerColumn, TextColumn, TimeElapsedColumn
    from rich.panel import Panel
    # Disable rich UI if we are in a CI environment to prevent log spam
    RICH_AVAILABLE = not is_ci
    console = Console(force_terminal=False if is_ci else None)
except ImportError:
    RICH_AVAILABLE = False
    if not is_ci:
        print("For a better experience, install rich: pip install rich")

def print_step(msg):
    if RICH_AVAILABLE:
        console.print(f"[bold cyan]>>[/bold cyan] {msg}")
    else:
        print(f">> {msg}")

def print_success(msg):
    if RICH_AVAILABLE:
        console.print(f"[bold green]✓[/bold green] {msg}")
    else:
        print(f"✓ {msg}")

def print_error(msg, output=""):
    if RICH_AVAILABLE:
        console.print(Panel(output, title="[bold red]Error Output", border_style="red"))
        console.print(f"[bold red]**** FAILURE: {msg} ***[/bold red]")
    else:
        print(f"**** FAILURE: {msg} ***\n{output}")

def setup_environment():
    # Set environment variables matching the original bash script
    os.environ["COMMON"] = "-DUSE_NO_DEFAULT=TRUE  -DUSE_CLANG=True   "
    user = os.environ.get("USER", os.environ.get("USERNAME", ""))
    
    cargo_bin = f"/home/{user}/.cargo/bin"
    local_bin = f"/home/{user}/.local/bin"
    current_path = os.environ.get("PATH", "")
    os.environ["PATH"] = f"{cargo_bin}:{local_bin}:{current_path}"
    
    os.environ["PICO_SDK"] = "/opt/pico/pico-sdk"
    os.environ["ROOT"] = os.getcwd()

def firmware_image(build_dir: Path):
    """The image the link step produced.

    LN (GD32/CH32) builds end with a checksummed `.elf.ck_bin`; RP2040/RP2350 have no
    checksum step, so their `.elf.bin` is the image. Prefer the checksummed one, and
    never report a `.uf2` (2x the size, its own container).
    """
    for pattern in ("*.ck_bin", "*.elf.bin", "*.bin"):
        images = sorted(build_dir.glob(pattern))
        if images:
            return images[0]
    return None


def runbuild(preset: str):
    cur_dir = Path.cwd()
    folder_name = f"build{preset}"
    build_dir = cur_dir / folder_name
    
    if build_dir.exists():
        shutil.rmtree(build_dir)
    build_dir.mkdir()
    
    cmake_cmd = ["cmake", "--preset", preset, ".."]
    make_cmd = ["make", "-j", "4"]
    
    try:
        subprocess.run(
            cmake_cmd, 
            cwd=build_dir, 
            check=True, 
            stdout=subprocess.PIPE, 
            stderr=subprocess.STDOUT, 
            text=True
        )
        subprocess.run(
            make_cmd, 
            cwd=build_dir, 
            check=True, 
            stdout=subprocess.PIPE, 
            stderr=subprocess.STDOUT, 
            text=True
        )
    except subprocess.CalledProcessError as e:
        print_error(preset, e.stdout)
        sys.exit(-1)
    return firmware_image(build_dir)


def describe_image(image):
    "Firmware size in kilobytes, e.g. '173.4 KB (swindle_....elf.ck_bin)'."
    if image is None:
        return "no .bin/.ck_bin produced"
    return f"{image.stat().st_size / 1024.0:.1f} KB ({image.name})"

def main():
    setup_environment()
    
    targets = [
        "rp2040",
        "rp2040_inv",
        "rp2350",
        "rp2350_inv",
        "gd32f303_48",
        "gd32f303_48_inv_ws2812",
        "ch32v307",
        "ch32v307_inv_ws2812",
        # "ch32v307_net",
    ]
    sizes = []
    
    if RICH_AVAILABLE:
        console.print(Panel("[bold blue]Starting CI Build Process[/bold blue]", border_style="blue"))
        
        with Progress(
            SpinnerColumn(),
            TextColumn("[progress.description]{task.description}"),
            TimeElapsedColumn(),
            console=console,
        ) as progress:
            overall_task = progress.add_task("[cyan]Overall Progress...", total=len(targets))
            
            for target in targets:
                task = progress.add_task(f"Building [yellow]{target}[/yellow]...", total=None)
                image = runbuild(target)
                sizes.append((target, image))
                progress.update(task, completed=1)
                progress.update(
                    task,
                    description=f"[green]Built [yellow]{target}[/yellow] "
                    f"[magenta]{describe_image(image)}[/magenta][/green]",
                )
                progress.update(overall_task, advance=1)
                
        console.print("\n[bold]Firmware sizes[/bold]")
        for target, image in sizes:
            console.print(
                f"  [yellow]{target:<24}[/yellow] [magenta]{describe_image(image)}[/magenta]"
            )
        console.print("\n[bold green]All builds completed successfully! 🎉[/bold green]\n")
    else:
        print("Starting CI Build Process")
        for i, target in enumerate(targets, 1):
            print_step(f"Building {target} ({i}/{len(targets)})...")
            image = runbuild(target)
            sizes.append((target, image))
            print_success(f"Built {target} - {describe_image(image)}")
        print("\nFirmware sizes")
        for target, image in sizes:
            print(f"  {target:<24} {describe_image(image)}")
        print("\nAll builds completed successfully!")

if __name__ == "__main__":
    main()
