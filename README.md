# yatta
BSP Tiling Window Manager for Windows 10

![demo](https://s2.gifyu.com/images/ezgif-1-a21b17f39d06.gif)

## Getting Started
This project is still heavily under development and there are no prebuilt binaries available yet.

If you would like to use `yatta`, you will need
a [working Rust development environment on Windows 10](https://rustup.rs/). If you are using
the `x86_64-pc-windows-msvc` toolchain, make sure you have also installed
the [Build Tools for Visual Studio 2019](https://stackoverflow.com/a/55603112).

You can then clone this repo and compile the source code to install the binaries
for `yatta` and `yattac`:

```powershell
cargo install --path yatta
cargo install --path yattac
```

By running `yattac start` at a Powershell prompt, you should see the following output:

```
Start-Process yatta -WindowStyle hidden
```

This means that `yatta` is now running in the background, tiling all your windows, and listening for commands sent to it
by `yattac`.

You can similarly stop the process by running `yattac stop`, and you should see the following output:

```
Stop-Process -Name yatta
```

## Keybindings

This project does not handle anything related to keybindings and keyboard shortcuts. I am currently
using [AutoHotKey](https://www.autohotkey.com/)
to manage my window management keyboard shortcuts. Here is a sample `yatta.ahk` AHK script that you can use as a
starting point for your own:

```ahk
!h::
Run, yattac.exe focus left, Hide
return

!j::
Run, yattac.exe focus down, Hide
return

!k::
Run, yattac.exe focus up, Hide
return

!l::
Run, yattac.exe focus right, Hide
return

!+h::
Run, yattac.exe move left, Hide
return

!+j::
Run, yattac.exe move down, Hide
return

!+k::
Run, yattac.exe move up, Hide
return

!+l::
Run, yattac.exe move right, Hide
return

!+Enter::
Run, yattac.exe promote, Hide
return

!+c::
Run, yattac.exe layout columns, Hide
return

!+t::
Run, yattac.exe layout bspv, Hide
return

!+r::
Run, yattac.exe retile, Hide
return

!+f::
Run, yattac.exe toggle-float, Hide
return

!p::
Run, yattac.exe toggle-pause, Hide
return
```

As more commands are still being added and some commands and arguments may change before the CLI is stabilised, I
recommend running `yattac.exe help` to see the full list of commands and operations available to be bound to keyboard
shortcuts.