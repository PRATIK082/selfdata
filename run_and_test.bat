@echo off
setlocal
cd /d "%~dp0"

echo === [1/7] Generate model (xsd -> Rust) ===
cargo run -p xsdgen --release -- schemas\AUTOSAR_00047.xsd crates\arxml-model-r4-0\src\generated.rs --namespace http://autosar.org/schema/r4.0
if errorlevel 1 goto :fail

echo === [2/7] Generate draft per-element API (xsd -> Rust) ===
cargo run -p xsd-api-gen -- schemas\AUTOSAR_00047.xsd crates\arxml-model-r4-0\src\element_api.rs
if errorlevel 1 goto :fail

echo === [3/7] Build workspace ===
cargo build --workspace
if errorlevel 1 goto :fail

echo === [4/7] Run tests ===
cargo test --workspace
if errorlevel 1 goto :fail

echo === [5/7] Run read_write_validate example ===
cargo run -p arxml --example read_write_validate
if errorlevel 1 goto :fail

echo === [6/7] Run generic_edit example ===
cargo run -p arxml-core --example generic_edit -- crates\arxml\examples\data\sample.arxml
if errorlevel 1 goto :fail

echo === [7/7] Build model index ===
cargo run -p arxml-index --release -- crates\arxml-model-r4-0\src\generated.rs -o target\model-index.json
if errorlevel 1 goto :fail

echo.
echo ALL STEPS PASSED
goto :end

:fail
echo.
echo FAILED - see output above
exit /b 1

:end
endlocal