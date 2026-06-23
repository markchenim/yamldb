package io.github.markchenim.yamldb.jdbc;

import io.github.markchenim.yamldb.jdbc.YamlDbJdbcProxies.YamlDbTable;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Base64;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.stream.Stream;

final class YamlDbSql {
    private static final String ROW_MARKER = "--YAMLDB-ROW--";
    private static Path bundledYq;

    private YamlDbSql() {
    }

    static YamlDbTable execute(Path path, String sql) throws SQLException {
        Query query = Query.parse(sql);
        Path tablePath = resolveTablePath(path, query.table());
        List<Map<String, Object>> rows = readYaml(tablePath).stream()
                .filter(query::matches)
                .toList();
        return YamlDbJdbcProxies.table(rows);
    }

    static List<String> tables(Path path) throws SQLException {
        if (Files.isRegularFile(path)) {
            return List.of("data");
        }
        if (!Files.isDirectory(path)) {
            return List.of();
        }

        try (Stream<Path> entries = Files.list(path)) {
            return entries
                    .filter(Files::isRegularFile)
                    .filter(YamlDbSql::isYamlPath)
                    .map(YamlDbSql::tableName)
                    .sorted(String.CASE_INSENSITIVE_ORDER)
                    .toList();
        } catch (IOException e) {
            throw new SQLException("Failed to list YAML directory: " + path, e);
        }
    }

    static YamlDbTable table(Path path, String table) throws SQLException {
        return YamlDbJdbcProxies.table(readYaml(resolveTablePath(path, table)));
    }

    static boolean isKnownTable(Path path, String table) {
        return resolveTablePathOrNull(path, table) != null;
    }

    private static Path resolveTablePath(Path source, String table) throws SQLException {
        Path path = resolveTablePathOrNull(source, table);
        if (path == null) {
            throw new SQLException("Unknown YAML table: " + table);
        }
        return path;
    }

    private static Path resolveTablePathOrNull(Path source, String table) {
        if (Files.isRegularFile(source)) {
            String sourceTable = tableName(source);
            if ("data".equalsIgnoreCase(table) || sourceTable.equalsIgnoreCase(table)) {
                return source;
            }
            return null;
        }

        if (!Files.isDirectory(source)) {
            return null;
        }

        for (String ext : List.of("yaml", "yml")) {
            Path candidate = source.resolve(table + "." + ext);
            if (Files.isRegularFile(candidate)) {
                return candidate;
            }
        }

        try (Stream<Path> entries = Files.list(source)) {
            return entries
                    .filter(Files::isRegularFile)
                    .filter(YamlDbSql::isYamlPath)
                    .filter(path -> tableName(path).equalsIgnoreCase(table))
                    .findFirst()
                    .orElse(null);
        } catch (IOException e) {
            return null;
        }
    }

    private static boolean isYamlPath(Path path) {
        String name = path.getFileName().toString().toLowerCase(Locale.ROOT);
        return name.endsWith(".yaml") || name.endsWith(".yml");
    }

    private static String tableName(Path path) {
        String name = path.getFileName().toString();
        int dot = name.lastIndexOf('.');
        return dot > 0 ? name.substring(0, dot) : name;
    }

    private static List<Map<String, Object>> readYaml(Path path) throws SQLException {
        if (!Files.exists(path)) {
            return List.of();
        }

        String output = runYq(path);
        if (output.trim().isEmpty()) {
            return List.of();
        }

        List<Map<String, Object>> rows = new ArrayList<>();
        Map<String, Object> current = null;
        for (String line : output.split("\\R", -1)) {
            if (line.isEmpty()) {
                continue;
            }
            if (ROW_MARKER.equals(line)) {
                if (current != null && !current.isEmpty()) {
                    rows.add(current);
                }
                current = new LinkedHashMap<>();
                continue;
            }
            if (current == null) {
                throw new SQLException("Invalid yq row output before row marker: " + line);
            }
            String[] parts = line.split("\\t", 2);
            if (parts.length != 2) {
                throw new SQLException("Invalid yq row output: " + line);
            }
            String key = decodeBase64(parts[0]);
            current.put(key, parseJsonValue(parts[1]));
        }
        if (current != null && !current.isEmpty()) {
            rows.add(current);
        }
        return rows;
    }

    private static String runYq(Path path) throws SQLException {
        String expression = ".[] | ([\"" + ROW_MARKER
                + "\"] + (to_entries | map(\"\\(.key | @base64)\\t\\(.value | @json)\")))[]";
        Process process;
        try {
            process = new ProcessBuilder(yqCommand(), "-r", expression, path.toString()).start();
        } catch (IOException e) {
            throw new SQLException("Failed to read YAML file: " + path, e);
        }

        try {
            byte[] stdout = process.getInputStream().readAllBytes();
            byte[] stderr = process.getErrorStream().readAllBytes();
            int status = process.waitFor();
            if (status != 0) {
                String message = new String(stderr, StandardCharsets.UTF_8).trim();
                throw new SQLException(message.isEmpty() ? "yq command failed" : message);
            }
            return new String(stdout, StandardCharsets.UTF_8);
        } catch (IOException e) {
            throw new SQLException("Failed to read yq output", e);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new SQLException("Interrupted while reading YAML file", e);
        }
    }

    private static String yqCommand() throws SQLException {
        String property = System.getProperty("yamldb.yq");
        if (property != null && !property.isBlank()) {
            return property;
        }

        String env = System.getenv("YAMLDB_YQ");
        if (env != null && !env.isBlank()) {
            return env;
        }

        Path bundled = bundledYq();
        if (bundled != null) {
            return bundled.toString();
        }

        return isWindows() ? "yq.exe" : "yq";
    }

    private static synchronized Path bundledYq() throws SQLException {
        if (bundledYq != null) {
            return bundledYq;
        }

        String resource = "/bin/" + platformName() + "/" + (isWindows() ? "yq.exe" : "yq");
        try (InputStream input = YamlDbSql.class.getResourceAsStream(resource)) {
            if (input == null) {
                return null;
            }
            Path dir = Files.createTempDirectory("yamldb-yq-");
            Path yq = dir.resolve(isWindows() ? "yq.exe" : "yq");
            Files.copy(input, yq, StandardCopyOption.REPLACE_EXISTING);
            yq.toFile().setExecutable(true, true);
            yq.toFile().deleteOnExit();
            dir.toFile().deleteOnExit();
            bundledYq = yq;
            return bundledYq;
        } catch (IOException e) {
            throw new SQLException("Failed to extract bundled yq", e);
        }
    }

    private static String platformName() {
        return osName() + "-" + archName();
    }

    private static String osName() {
        String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
        if (os.contains("win")) {
            return "windows";
        }
        if (os.contains("mac") || os.contains("darwin")) {
            return "macos";
        }
        return "linux";
    }

    private static String archName() {
        String arch = System.getProperty("os.arch", "").toLowerCase(Locale.ROOT);
        if (arch.equals("x86_64") || arch.equals("amd64")) {
            return "amd64";
        }
        if (arch.equals("aarch64") || arch.equals("arm64")) {
            return "arm64";
        }
        return arch;
    }

    private static boolean isWindows() {
        return osName().equals("windows");
    }

    private static String decodeBase64(String value) throws SQLException {
        try {
            return new String(Base64.getDecoder().decode(value), StandardCharsets.UTF_8);
        } catch (IllegalArgumentException e) {
            throw new SQLException("Invalid yq base64 output", e);
        }
    }

    private static Object parseJsonValue(String value) throws SQLException {
        String trimmed = value.trim();
        if (trimmed.equals("null")) {
            return null;
        }
        if (trimmed.equals("true") || trimmed.equals("false")) {
            return Boolean.parseBoolean(trimmed);
        }
        if (trimmed.startsWith("\"") && trimmed.endsWith("\"")) {
            return parseJsonString(trimmed);
        }
        try {
            return Long.parseLong(trimmed);
        } catch (NumberFormatException ignored) {
        }
        try {
            return Double.parseDouble(trimmed);
        } catch (NumberFormatException ignored) {
        }
        return trimmed;
    }

    private static String parseJsonString(String value) throws SQLException {
        StringBuilder result = new StringBuilder();
        for (int i = 1; i < value.length() - 1; i++) {
            char ch = value.charAt(i);
            if (ch != '\\') {
                result.append(ch);
                continue;
            }
            if (++i >= value.length() - 1) {
                throw new SQLException("Invalid JSON string: " + value);
            }
            char escaped = value.charAt(i);
            switch (escaped) {
                case '"', '\\', '/' -> result.append(escaped);
                case 'b' -> result.append('\b');
                case 'f' -> result.append('\f');
                case 'n' -> result.append('\n');
                case 'r' -> result.append('\r');
                case 't' -> result.append('\t');
                case 'u' -> {
                    if (i + 4 >= value.length()) {
                        throw new SQLException("Invalid JSON unicode escape: " + value);
                    }
                    String hex = value.substring(i + 1, i + 5);
                    try {
                        result.append((char) Integer.parseInt(hex, 16));
                    } catch (NumberFormatException e) {
                        throw new SQLException("Invalid JSON unicode escape: " + value, e);
                    }
                    i += 4;
                }
                default -> throw new SQLException("Invalid JSON escape: " + value);
            }
        }
        return result.toString();
    }

    private interface Condition {
        boolean matches(Map<String, Object> row);
    }

    private record Query(String table, Condition condition) {
        static Query parse(String sql) throws SQLException {
            String trimmed = sql.trim();
            if (trimmed.endsWith(";")) {
                trimmed = trimmed.substring(0, trimmed.length() - 1).trim();
            }
            String lower = trimmed.toLowerCase(Locale.ROOT);
            if (!lower.startsWith("select")) {
                throw new SQLException("Only SELECT queries are supported");
            }
            if (!lower.matches("select\\s+\\*\\s+from\\s+[`\\\"a-zA-Z0-9_\\-.]+(\\s+where\\s+.+)?")) {
                throw new SQLException("Only SELECT * FROM <yaml_table> queries are supported");
            }
            int where = lower.indexOf(" where ");
            String select = where < 0 ? trimmed : trimmed.substring(0, where);
            String[] tokens = select.split("\\s+");
            if (tokens.length != 4) {
                throw new SQLException("Only SELECT * FROM <yaml_table> queries are supported");
            }
            String table = unquoteIdentifier(tokens[3]);
            if (where < 0) {
                return new Query(table, row -> true);
            }
            return new Query(table, parseCondition(trimmed.substring(where + 7).trim()));
        }

        boolean matches(Map<String, Object> row) {
            return condition.matches(row);
        }
    }

    private static String unquoteIdentifier(String value) throws SQLException {
        if (value.isEmpty()) {
            throw new SQLException("Missing table name");
        }
        if ((value.startsWith("\"") && value.endsWith("\""))
                || (value.startsWith("`") && value.endsWith("`"))) {
            return value.substring(1, value.length() - 1);
        }
        return value;
    }

    private static Condition parseCondition(String condition) throws SQLException {
        String lower = condition.toLowerCase(Locale.ROOT);
        int and = lower.indexOf(" and ");
        if (and >= 0) {
            Condition left = parseComparison(condition.substring(0, and));
            Condition right = parseComparison(condition.substring(and + 5));
            return row -> left.matches(row) && right.matches(row);
        }
        int or = lower.indexOf(" or ");
        if (or >= 0) {
            Condition left = parseComparison(condition.substring(0, or));
            Condition right = parseComparison(condition.substring(or + 4));
            return row -> left.matches(row) || right.matches(row);
        }
        return parseComparison(condition);
    }

    private static Condition parseComparison(String comparison) throws SQLException {
        for (String op : List.of(">=", "<=", "!=", "=", ">", "<")) {
            int index = comparison.indexOf(op);
            if (index > 0) {
                String key = comparison.substring(0, index).trim();
                Object expected = parseSqlLiteral(comparison.substring(index + op.length()).trim());
                return row -> compare(row.get(key), expected, op);
            }
        }
        throw new SQLException("Invalid WHERE condition: " + comparison);
    }

    private static Object parseSqlLiteral(String value) {
        if (value.isEmpty() || value.equalsIgnoreCase("null") || value.equals("~")) {
            return null;
        }
        if ((value.startsWith("'") && value.endsWith("'"))
                || (value.startsWith("\"") && value.endsWith("\""))) {
            return value.substring(1, value.length() - 1);
        }
        if (value.equalsIgnoreCase("true") || value.equalsIgnoreCase("false")) {
            return Boolean.parseBoolean(value);
        }
        try {
            return Long.parseLong(value);
        } catch (NumberFormatException ignored) {
        }
        try {
            return Double.parseDouble(value);
        } catch (NumberFormatException ignored) {
        }
        return value;
    }

    @SuppressWarnings({ "unchecked", "rawtypes" })
    private static boolean compare(Object actual, Object expected, String op) {
        if ("=".equals(op)) {
            return valuesEqual(actual, expected);
        }
        if ("!=".equals(op)) {
            return !valuesEqual(actual, expected);
        }
        if (actual == null || expected == null) {
            return false;
        }
        int result;
        if (actual instanceof Number && expected instanceof Number) {
            result = Double.compare(((Number) actual).doubleValue(), ((Number) expected).doubleValue());
        } else if (actual instanceof Comparable && expected instanceof Comparable) {
            result = ((Comparable) actual.toString()).compareTo(expected.toString());
        } else {
            return false;
        }
        return switch (op) {
            case ">" -> result > 0;
            case "<" -> result < 0;
            case ">=" -> result >= 0;
            case "<=" -> result <= 0;
            default -> false;
        };
    }

    private static boolean valuesEqual(Object actual, Object expected) {
        if (actual instanceof Number && expected instanceof Number) {
            return Double.compare(((Number) actual).doubleValue(), ((Number) expected).doubleValue()) == 0;
        }
        return actual == null ? expected == null : actual.equals(expected);
    }
}
