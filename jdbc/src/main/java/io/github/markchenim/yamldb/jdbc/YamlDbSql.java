package io.github.markchenim.yamldb.jdbc;

import io.github.markchenim.yamldb.jdbc.YamlDbJdbcProxies.YamlDbTable;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;

final class YamlDbSql {
    private YamlDbSql() {
    }

    static YamlDbTable execute(Path path, String sql) throws SQLException {
        Query query = Query.parse(sql);
        List<Map<String, Object>> rows = readYaml(path).stream()
                .filter(query::matches)
                .toList();
        return YamlDbJdbcProxies.table(rows);
    }

    private static List<Map<String, Object>> readYaml(Path path) throws SQLException {
        List<String> lines;
        try {
            lines = Files.readAllLines(path);
        } catch (IOException e) {
            throw new SQLException("Failed to read YAML file: " + path, e);
        }

        List<Map<String, Object>> rows = new ArrayList<>();
        Map<String, Object> current = null;
        for (String line : lines) {
            String trimmed = line.trim();
            if (trimmed.isEmpty() || trimmed.startsWith("#")) {
                continue;
            }
            if (trimmed.startsWith("- ")) {
                current = new LinkedHashMap<>();
                rows.add(current);
                String rest = trimmed.substring(2).trim();
                if (!rest.isEmpty()) {
                    putScalar(current, rest);
                }
            } else if (current != null) {
                putScalar(current, trimmed);
            }
        }
        return rows;
    }

    private static void putScalar(Map<String, Object> row, String line) {
        int colon = line.indexOf(':');
        if (colon <= 0) {
            return;
        }
        String key = line.substring(0, colon).trim();
        String value = line.substring(colon + 1).trim();
        row.put(key, parseScalar(value));
    }

    private static Object parseScalar(String value) {
        if (value.isEmpty() || value.equals("null") || value.equals("~")) {
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

    private interface Condition {
        boolean matches(Map<String, Object> row);
    }

    private record Query(Condition condition) {
        static Query parse(String sql) throws SQLException {
            String trimmed = sql.trim();
            String lower = trimmed.toLowerCase(Locale.ROOT);
            if (!lower.startsWith("select")) {
                throw new SQLException("Only SELECT queries are supported");
            }
            if (!lower.matches("select\\s+\\*\\s+from\\s+data(\\s+where\\s+.+)?")) {
                throw new SQLException("Only SELECT * FROM data queries are supported");
            }
            int where = lower.indexOf(" where ");
            if (where < 0) {
                return new Query(row -> true);
            }
            return new Query(parseCondition(trimmed.substring(where + 7).trim()));
        }

        boolean matches(Map<String, Object> row) {
            return condition.matches(row);
        }
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
                Object expected = parseScalar(comparison.substring(index + op.length()).trim());
                return row -> compare(row.get(key), expected, op);
            }
        }
        throw new SQLException("Invalid WHERE condition: " + comparison);
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
