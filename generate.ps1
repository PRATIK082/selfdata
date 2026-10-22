<#
.SYNOPSIS
    Regenerates the Rust model of an AUTOSAR XSD version with the xsd-parser
    based generator in tools/xsdgen.

.EXAMPLE
    ./generate.ps1
    ./generate.ps1 -Schema schemas/AUTOSAR_00047.xsd -Namespace http://autosar.org/schema/r4.0 -Output crates/arxml-model-r4-0/src/generated.rs

.NOTES
    The generator needs the W3C xml.xsd (schemas/xml.xsd) next to the AUTOSAR
    schema because AUTOSAR imports the XML namespace.
#>
[CmdletBinding()]
param(
    [string]$Schema = "schemas/AUTOSAR_00047.xsd",
    [string]$Namespace = "http://autosar.org/schema/r4.0",
    [string]$Output = "crates/arxml-model-r4-0/src/generated.rs",
    [switch]$NoNamespaceModules,
    [switch]$SchemaModules,
    [switch]$Debug
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $root

try {
    $args = @("run", "-p", "xsdgen", "--release", "--", $Schema, $Output, "--namespace", $Namespace)

    if ($NoNamespaceModules) {
        $args += "--no-namespace-modules"
    }
    if ($SchemaModules) {
        $args += "--schema-modules"
    }
    if ($Debug) {
        $args += "--debug"
    }

    Write-Host "> cargo $($args -join ' ')"
    & cargo @args
    if ($LASTEXITCODE -ne 0) {
        throw "xsdgen failed with exit code $LASTEXITCODE"
    }
}
finally {
    Pop-Location
}
