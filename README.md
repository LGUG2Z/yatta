# yatta

BSP Tiling Window Manager for Windows 10

![demo](https://s2.gifyu.com/images/ezgif-1-a21b17f39d06.gif)

## Getting Started

This project is still heavily under development and there are no prebuilt binaries available yet.

If you would like to use `yatta`, you will need
a [working Rust development environment on Windows 10](https://rustup.rs/). If you are using
the `x86_64-pc-windows-msvc` toolchain, make sure you have also installed
the [Build Tools for Visual Studio 2019](https://stackoverflow.com/a/55603112).

You can then clone this repo and compile the source code to install the binaries for `yatta` and `yattac`:

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
; Start yatta, this command makes sure no duplicate processes will be created
Run, yattac.exe start, , Hide

; Send the configuration options for yatta here

; Always float IntelliJ popups, matching on class
Run, yattac.exe float-class SunAwtDialog, , Hide

; Always float Control Panel, matching on title
Run, yattac.exe float-title "Control Panel", , Hide

; Always float Task Manager, matching on class
Run, yattac.exe float-class TaskManagerWindow, , Hide

; Always float Wally, matching on executable name
Run, yattac.exe float-exe Wally.exe, , Hide

; Always float Calculator app, matching on window title
Run, yattac.exe float-title Calculator, , Hide
Run, yattac.exe float-exe 1Password.exe, , Hide

; Change the focused window, Alt + Vim direction keys
!h::
; This sends an Alt key which is a hack to steal focus when Windows doesn't feel like respecting SetForegroundWindow
; https://stackoverflow.com/questions/10740346/setforegroundwindow-only-working-while-visual-studio-is-open
Send !
Run, yattac.exe focus left, , Hide
return

!j::
Send !
Run, yattac.exe focus down, , Hide
return

!k::
Send !
Run, yattac.exe focus up, , Hide
return

!l::
Send !
Run, yattac.exe focus right, , Hide
return

; Move the focused window in a given direction, Alt + Shift + Vim direction keys
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

; Increase the size of a given edge in the BSPV and BSPH layouts, Alt + Arrow Key
!Left::
Run, yattac.exe resize left increase, Hide
return

!Right::
Run, yattac.exe resize right increase, Hide
return

!Up::
Run, yattac.exe resize top increase, Hide
return

!Down::
Run, yattac.exe resize bottom increase, Hide
return

; Decrease the size of a given edge in the BSPV and BSPH layouts, Alt + Shift + Arrow Key
!+Left::
Run, yattac.exe resize left decrease, Hide
return

!+Right::
Run, yattac.exe resize right decrease, Hide
return

!+Up::
Run, yattac.exe resize top decrease, Hide
return

!+Down::
Run, yattac.exe resize bottom decrease, Hide
return

; Move the focused window to the previous display, Alt + Shift + Left
!+Left::
Run, yattac.exe move-to-display previous, Hide
return

; Move the focused window to the next display, Alt + Shift + Right
!+Right::
Run, yattac.exe move-to-display next, Hide
return

; Promote the focused window to the top of the tree, Alt + Shift + Enter
!+Enter::
Run, yattac.exe promote, Hide
return

; Switch to an equal-width, max-height column layout, Alt + Shift + C
!+c::
Run, yattac.exe layout columns, Hide
return

; Switch to the default vertical bsp tiling layout, Alt + Shift + T
!+t::
Run, yattac.exe layout bspv, Hide
return

; Toggle the Monocle layout, Alt + Shift + F
!+f::
Run, yattac.exe toggle-monocle, Hide
return

; Force a retile if things get janky, Alt + Shift + R
!+r::
Run, yattac.exe retile, Hide
return

; Float the focused window, Alt + T
!t::
Run, yattac.exe toggle-float, Hide
return

; Pause responding to any window events or yattac commands, Alt + P
!p::
Run, yattac.exe toggle-pause, Hide
return

; Switch to workspace
!1::
Run, yattac.exe set-workspace 0, Hide
return

!2::
Run, yattac.exe set-workspace 1, Hide
return

!3::
Run, yattac.exe set-workspace 2, Hide
return

!4::
Run, yattac.exe set-workspace 3, Hide
return

!5::
Run, yattac.exe set-workspace 4, Hide
return

!6::
Run, yattac.exe set-workspace 5, Hide
return

!7::
Run, yattac.exe set-workspace 6, Hide
return

!8::
Run, yattac.exe set-workspace 7, Hide
return

!9::
Run, yattac.exe set-workspace 8, Hide
return

; Move window to workspace
!+1::
Run, yattac.exe move-window-to-workspace 0, Hide
return

!+2::
Run, yattac.exe move-window-to-workspace 1, Hide
return

!+3::
Run, yattac.exe move-window-to-workspace 2, Hide
return

!+4::
Run, yattac.exe move-window-to-workspace 3, Hide
return

!+5::
Run, yattac.exe move-window-to-workspace 4, Hide
return

!+6::
Run, yattac.exe move-window-to-workspace 5, Hide
return

!+7::
Run, yattac.exe move-window-to-workspace 6, Hide
return

!+8::
Run, yattac.exe move-window-to-workspace 7, Hide
return

!+9::
Run, yattac.exe move-window-to-workspace 8, Hide
return
```

As more commands are still being added and some commands and arguments may change before the CLI is stabilised, I
recommend running `yattac.exe help` to see the full list of commands and operations available to be bound to keyboard
shortcuts.
