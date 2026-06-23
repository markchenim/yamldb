$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$BuildDir = Join-Path $Root "target"
$ClassesDir = Join-Path $BuildDir "classes"
$TestClassesDir = Join-Path $BuildDir "test-classes"
$JarPath = Join-Path $BuildDir "yamldb-jdbc.jar"

Remove-Item -Recurse -Force $BuildDir -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $ClassesDir, $TestClassesDir | Out-Null

$Sources = Get-ChildItem -Path (Join-Path $Root "src/main/java") -Recurse -Filter *.java | ForEach-Object { $_.FullName }
javac -encoding UTF-8 -d $ClassesDir $Sources

Copy-Item -Recurse -Force (Join-Path $Root "src/main/resources/*") $ClassesDir
jar --create --file $JarPath -C $ClassesDir .

$TestSources = Get-ChildItem -Path (Join-Path $Root "src/test/java") -Recurse -Filter *.java | ForEach-Object { $_.FullName }
javac -encoding UTF-8 -cp $ClassesDir -d $TestClassesDir $TestSources
java -cp "$ClassesDir;$TestClassesDir" io.github.markchenim.yamldb.jdbc.YamlDbJdbcDriverTest

Write-Host "Built $JarPath"
