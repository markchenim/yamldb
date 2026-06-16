#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="$ROOT/target"
CLASSES_DIR="$BUILD_DIR/classes"
TEST_CLASSES_DIR="$BUILD_DIR/test-classes"
JAR_PATH="$BUILD_DIR/yamldb-jdbc.jar"

rm -rf "$BUILD_DIR"
mkdir -p "$CLASSES_DIR" "$TEST_CLASSES_DIR"

find "$ROOT/src/main/java" -name '*.java' -print0 | xargs -0 javac -encoding UTF-8 -d "$CLASSES_DIR"
cp -R "$ROOT/src/main/resources/META-INF" "$CLASSES_DIR/"
jar --create --file "$JAR_PATH" -C "$CLASSES_DIR" .

find "$ROOT/src/test/java" -name '*.java' -print0 | xargs -0 javac -encoding UTF-8 -cp "$CLASSES_DIR" -d "$TEST_CLASSES_DIR"
java -cp "$CLASSES_DIR:$TEST_CLASSES_DIR" io.github.markchenim.yamldb.jdbc.YamlDbJdbcDriverTest

echo "Built $JAR_PATH"
