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
                runbuild(target)
                progress.update(task, completed=1)
                progress.update(task, description=f"[green]Built [yellow]{target}[/yellow][/green]")
                progress.update(overall_task, advance=1)
                
        console.print("\n[bold green]All builds completed successfully! 🎉[/bold green]\n")
    else:
        print("Starting CI Build Process")
        for i, target in enumerate(targets, 1):
            print_step(f"Building {target} ({i}/{len(targets)})...")
            runbuild(target)
            print_success(f"Built {target}")
        print("\nAll builds completed successfully!")

if __name__ == "__main__":
    main()
