@echo off
setlocal
cd /d "%~dp0"

echo Cleaning project debug and compiled artifacts...
if exist target rmdir /s /q target
if exist crates\arxml-model-r4-0\src\generated.rs del /f /q crates\arxml-model-r4-0\src\generated.rs
if exist crates\arxml-model-r4-0\src\element_api.rs del /f /q crates\arxml-model-r4-0\src\element_api.rs
del /s /q *.log 2>nul
del /s /q *.arxml.out 2>nul
del /s /q sample.out.arxml 2>nul
del /s /q generic-edit.arxml 2>nul
del /s /q model-index.json 2>nul

echo Cleaning Cargo registry/git caches...
if exist "%USERPROFILE%\.cargo\registry\cache" rmdir /s /q "%USERPROFILE%\.cargo\registry\cache"
if exist "%USERPROFILE%\.cargo\registry\src" rmdir /s /q "%USERPROFILE%\.cargo\registry\src"
if exist "%USERPROFILE%\.cargo\git\checkouts" rmdir /s /q "%USERPROFILE%\.cargo\git\checkouts"
if exist "%USERPROFILE%\.cargo\git\db" rmdir /s /q "%USERPROFILE%\.cargo\git\db"

echo Cleaning temporary files...
if exist "%LOCALAPPDATA%\Temp\opencode" rmdir /s /q "%LOCALAPPDATA%\Temp\opencode"

echo Removing optional Rust docs component (if present)...
rustup component remove rust-docs 2>nul

echo Clean complete.
endlocal