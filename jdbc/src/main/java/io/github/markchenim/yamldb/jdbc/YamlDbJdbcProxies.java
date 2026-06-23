package io.github.markchenim.yamldb.jdbc;

import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Proxy;
import java.nio.file.Path;
import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.sql.Statement;
import java.sql.Types;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;

final class YamlDbJdbcProxies {
    private YamlDbJdbcProxies() {
    }

    static Connection connection(String path) {
        InvocationHandler handler = new ConnectionHandler(Path.of(path));
        return proxy(Connection.class, handler);
    }

    @SuppressWarnings("unchecked")
    private static <T> T proxy(Class<T> iface, InvocationHandler handler) {
        return (T) Proxy.newProxyInstance(
                iface.getClassLoader(),
                new Class<?>[] { iface },
                handler);
    }

    private static final class ConnectionHandler implements InvocationHandler {
        private final Path path;
        private boolean closed;

        private ConnectionHandler(Path path) {
            this.path = path;
        }

        @Override
        public Object invoke(Object proxy, Method method, Object[] args) throws Throwable {
            String name = method.getName();
            switch (name) {
                case "createStatement":
                    ensureOpen();
                    return statement((Connection) proxy, path);
                case "close":
                    closed = true;
                    return null;
                case "isClosed":
                    return closed;
                case "isValid":
                    return !closed;
                case "getAutoCommit":
                    return true;
                case "setAutoCommit":
                case "commit":
                case "rollback":
                case "setReadOnly":
                    ensureOpen();
                    return null;
                case "isReadOnly":
                    return true;
                case "nativeSQL":
                    ensureOpen();
                    return args[0];
                case "getMetaData":
                    ensureOpen();
                    return databaseMetaData((Connection) proxy, path);
                case "getCatalog":
                case "getSchema":
                    return null;
                case "unwrap":
                    return unwrap(proxy, args[0]);
                case "isWrapperFor":
                    return ((Class<?>) args[0]).isInstance(proxy);
                case "toString":
                    return "YamlDbConnection[" + path + "]";
                default:
                    throw unsupported(method);
            }
        }

        private void ensureOpen() throws SQLException {
            if (closed) {
                throw new SQLException("Connection is closed");
            }
        }
    }

    private static Statement statement(Connection connection, Path path) {
        InvocationHandler handler = new StatementHandler(connection, path);
        return proxy(Statement.class, handler);
    }

    private static DatabaseMetaData databaseMetaData(Connection connection, Path path) {
        InvocationHandler handler = new DatabaseMetaDataHandler(connection, path);
        return proxy(DatabaseMetaData.class, handler);
    }

    private static final class DatabaseMetaDataHandler implements InvocationHandler {
        private final Connection connection;
        private final Path path;

        private DatabaseMetaDataHandler(Connection connection, Path path) {
            this.connection = connection;
            this.path = path;
        }

        @Override
        public Object invoke(Object proxy, Method method, Object[] args) throws Throwable {
            String name = method.getName();
            switch (name) {
                case "getConnection":
                    return connection;
                case "getDatabaseProductName":
                    return "YamlDB";
                case "getDatabaseProductVersion":
                    return "0.10.0";
                case "getDriverName":
                    return "YamlDB JDBC Driver";
                case "getDriverVersion":
                    return "0.10.0";
                case "getDriverMajorVersion":
                    return 0;
                case "getDriverMinorVersion":
                    return 10;
                case "supportsResultSetType":
                    return (Integer) args[0] == ResultSet.TYPE_FORWARD_ONLY;
                case "supportsResultSetConcurrency":
                    return (Integer) args[0] == ResultSet.TYPE_FORWARD_ONLY
                            && (Integer) args[1] == ResultSet.CONCUR_READ_ONLY;
                case "isReadOnly":
                    return true;
                case "getTables":
                    return tablesResult((String) args[2]);
                case "getColumns":
                    return columnsResult((String) args[2], (String) args[3]);
                case "unwrap":
                    return unwrap(proxy, args[0]);
                case "isWrapperFor":
                    return ((Class<?>) args[0]).isInstance(proxy);
                case "toString":
                    return "YamlDbDatabaseMetaData[" + path + "]";
                default:
                    return defaultValue(method);
            }
        }

        private ResultSet tablesResult(String tablePattern) throws SQLException {
            List<Map<String, Object>> rows = new ArrayList<>();
            for (String table : YamlDbSql.tables(path)) {
                if (!matchesPattern(table, tablePattern)) {
                    continue;
                }
                Map<String, Object> row = new LinkedHashMap<>();
                row.put("TABLE_CAT", null);
                row.put("TABLE_SCHEM", null);
                row.put("TABLE_NAME", table);
                row.put("TABLE_TYPE", "TABLE");
                row.put("REMARKS", null);
                rows.add(row);
            }
            return resultSet(table(rows), 0);
        }

        private ResultSet columnsResult(String tablePattern, String columnPattern) throws SQLException {
            List<Map<String, Object>> rows = new ArrayList<>();
            for (String table : YamlDbSql.tables(path)) {
                if (!matchesPattern(table, tablePattern)) {
                    continue;
                }
                YamlDbTable yamlTable = YamlDbSql.table(path, table);
                int position = 1;
                for (String column : yamlTable.columns()) {
                    int type = inferSqlType(column, yamlTable.rows());
                    Map<String, Object> row = new LinkedHashMap<>();
                    row.put("TABLE_CAT", null);
                    row.put("TABLE_SCHEM", null);
                    row.put("TABLE_NAME", table);
                    row.put("COLUMN_NAME", column);
                    row.put("DATA_TYPE", type);
                    row.put("TYPE_NAME", sqlTypeName(type));
                    row.put("COLUMN_SIZE", 0);
                    row.put("NULLABLE", DatabaseMetaData.columnNullable);
                    row.put("ORDINAL_POSITION", position++);
                    row.put("IS_NULLABLE", "YES");
                    if (matchesPattern(column, columnPattern)) {
                        rows.add(row);
                    }
                }
            }
            return resultSet(table(rows), 0);
        }
    }

    private static final class StatementHandler implements InvocationHandler {
        private final Connection connection;
        private final Path path;
        private ResultSet resultSet;
        private int maxRows;
        private boolean closed;

        private StatementHandler(Connection connection, Path path) {
            this.connection = connection;
            this.path = path;
        }

        @Override
        public Object invoke(Object proxy, Method method, Object[] args) throws Throwable {
            String name = method.getName();
            switch (name) {
                case "executeQuery":
                    ensureOpen();
                    resultSet = resultSet(YamlDbSql.execute(path, (String) args[0]), maxRows);
                    return resultSet;
                case "execute":
                    ensureOpen();
                    resultSet = resultSet(YamlDbSql.execute(path, (String) args[0]), maxRows);
                    return true;
                case "getResultSet":
                    ensureOpen();
                    return resultSet;
                case "getUpdateCount":
                    return -1;
                case "setMaxRows":
                    maxRows = (Integer) args[0];
                    return null;
                case "getMaxRows":
                    return maxRows;
                case "close":
                    closed = true;
                    if (resultSet != null) {
                        resultSet.close();
                    }
                    return null;
                case "isClosed":
                    return closed;
                case "getConnection":
                    return connection;
                case "closeOnCompletion":
                    return null;
                case "isCloseOnCompletion":
                    return false;
                case "unwrap":
                    return unwrap(proxy, args[0]);
                case "isWrapperFor":
                    return ((Class<?>) args[0]).isInstance(proxy);
                case "toString":
                    return "YamlDbStatement[" + path + "]";
                default:
                    throw unsupported(method);
            }
        }

        private void ensureOpen() throws SQLException {
            if (closed) {
                throw new SQLException("Statement is closed");
            }
        }
    }

    private static ResultSet resultSet(YamlDbTable table, int maxRows) {
        List<Map<String, Object>> rows = table.rows();
        if (maxRows > 0 && rows.size() > maxRows) {
            rows = new ArrayList<>(rows.subList(0, maxRows));
        }
        InvocationHandler handler = new ResultSetHandler(table.columns(), rows);
        return proxy(ResultSet.class, handler);
    }

    private static final class ResultSetHandler implements InvocationHandler {
        private final List<String> columns;
        private final List<Map<String, Object>> rows;
        private int cursor = -1;
        private boolean lastWasNull;
        private boolean closed;

        private ResultSetHandler(List<String> columns, List<Map<String, Object>> rows) {
            this.columns = columns;
            this.rows = rows;
        }

        @Override
        public Object invoke(Object proxy, Method method, Object[] args) throws Throwable {
            String name = method.getName();
            switch (name) {
                case "next":
                    ensureOpen();
                    if (cursor + 1 < rows.size()) {
                        cursor++;
                        return true;
                    }
                    cursor = rows.size();
                    return false;
                case "getString":
                    Object stringValue = value(args[0]);
                    return stringValue == null ? null : stringValue.toString();
                case "getObject":
                    return value(args[0]);
                case "getInt":
                    return asNumber(value(args[0])).intValue();
                case "getLong":
                    return asNumber(value(args[0])).longValue();
                case "getDouble":
                    return asNumber(value(args[0])).doubleValue();
                case "getBoolean":
                    Object boolValue = value(args[0]);
                    if (boolValue == null) {
                        return false;
                    }
                    if (boolValue instanceof Boolean b) {
                        return b;
                    }
                    return Boolean.parseBoolean(boolValue.toString());
                case "wasNull":
                    return lastWasNull;
                case "findColumn":
                    return columnIndex((String) args[0]) + 1;
                case "getMetaData":
                    return metaData(columns, rows);
                case "getRow":
                    return cursor >= 0 && cursor < rows.size() ? cursor + 1 : 0;
                case "isBeforeFirst":
                    return cursor < 0 && !rows.isEmpty();
                case "isAfterLast":
                    return cursor >= rows.size() && !rows.isEmpty();
                case "close":
                    closed = true;
                    return null;
                case "isClosed":
                    return closed;
                case "unwrap":
                    return unwrap(proxy, args[0]);
                case "isWrapperFor":
                    return ((Class<?>) args[0]).isInstance(proxy);
                case "toString":
                    return "YamlDbResultSet[rows=" + rows.size() + "]";
                default:
                    throw unsupported(method);
            }
        }

        private Object value(Object key) throws SQLException {
            ensureOpen();
            if (cursor < 0 || cursor >= rows.size()) {
                throw new SQLException("Cursor is not positioned on a row");
            }
            Object value;
            if (key instanceof Integer index) {
                int column = index - 1;
                if (column < 0 || column >= columns.size()) {
                    throw new SQLException("Invalid column index: " + index);
                }
                value = rows.get(cursor).get(columns.get(column));
            } else {
                value = rows.get(cursor).get(columns.get(columnIndex((String) key)));
            }
            lastWasNull = value == null;
            return value;
        }

        private int columnIndex(String label) throws SQLException {
            for (int i = 0; i < columns.size(); i++) {
                if (columns.get(i).equalsIgnoreCase(label)) {
                    return i;
                }
            }
            throw new SQLException("Unknown column: " + label);
        }

        private Number asNumber(Object value) {
            if (value instanceof Number number) {
                return number;
            }
            if (value == null) {
                return 0;
            }
            String text = value.toString();
            return text.contains(".") ? Double.parseDouble(text) : Long.parseLong(text);
        }

        private void ensureOpen() throws SQLException {
            if (closed) {
                throw new SQLException("ResultSet is closed");
            }
        }
    }

    private static ResultSetMetaData metaData(List<String> columns, List<Map<String, Object>> rows) {
        InvocationHandler handler = new MetaDataHandler(columns, rows);
        return proxy(ResultSetMetaData.class, handler);
    }

    private static final class MetaDataHandler implements InvocationHandler {
        private final List<String> columns;
        private final List<Map<String, Object>> rows;

        private MetaDataHandler(List<String> columns, List<Map<String, Object>> rows) {
            this.columns = columns;
            this.rows = rows;
        }

        @Override
        public Object invoke(Object proxy, Method method, Object[] args) throws Throwable {
            String name = method.getName();
            switch (name) {
                case "getColumnCount":
                    return columns.size();
                case "getColumnName":
                case "getColumnLabel":
                    return column((Integer) args[0]);
                case "getColumnType":
                    return sqlType(column((Integer) args[0]));
                case "getColumnTypeName":
                    return sqlTypeName(sqlType(column((Integer) args[0])));
                case "getColumnClassName":
                    return columnClassName(column((Integer) args[0]));
                case "isNullable":
                    return ResultSetMetaData.columnNullable;
                case "isAutoIncrement":
                case "isCurrency":
                case "isDefinitelyWritable":
                case "isReadOnly":
                case "isSearchable":
                case "isSigned":
                case "isWritable":
                    return false;
                case "getPrecision":
                case "getScale":
                case "getColumnDisplaySize":
                    return 0;
                case "getCatalogName":
                case "getSchemaName":
                case "getTableName":
                    return "";
                case "unwrap":
                    return unwrap(proxy, args[0]);
                case "isWrapperFor":
                    return ((Class<?>) args[0]).isInstance(proxy);
                default:
                    throw unsupported(method);
            }
        }

        private String column(int index) throws SQLException {
            int column = index - 1;
            if (column < 0 || column >= columns.size()) {
                throw new SQLException("Invalid column index: " + index);
            }
            return columns.get(column);
        }

        private int sqlType(String column) {
            return inferSqlType(column, rows);
        }

        private String columnClassName(String column) {
            for (Map<String, Object> row : rows) {
                Object value = row.get(column);
                if (value != null) {
                    return value.getClass().getName();
                }
            }
            return String.class.getName();
        }
    }

    private static int inferSqlType(String column, List<Map<String, Object>> rows) {
        for (Map<String, Object> row : rows) {
            Object value = row.get(column);
            if (value instanceof Integer || value instanceof Long) {
                return Types.BIGINT;
            }
            if (value instanceof Float || value instanceof Double) {
                return Types.DOUBLE;
            }
            if (value instanceof Boolean) {
                return Types.BOOLEAN;
            }
        }
        return Types.VARCHAR;
    }

    private static String sqlTypeName(int type) {
        return switch (type) {
            case Types.BIGINT -> "BIGINT";
            case Types.DOUBLE -> "DOUBLE";
            case Types.BOOLEAN -> "BOOLEAN";
            default -> "VARCHAR";
        };
    }

    private static boolean matchesPattern(String value, String pattern) {
        if (pattern == null || pattern.isEmpty() || "%".equals(pattern)) {
            return true;
        }
        String regex = pattern
                .replace("\\", "\\\\")
                .replace(".", "\\.")
                .replace("%", ".*")
                .replace("_", ".");
        return value.matches("(?i)" + regex);
    }

    private static Object defaultValue(Method method) throws SQLFeatureNotSupportedException {
        Class<?> type = method.getReturnType();
        if (type == Boolean.TYPE) {
            return false;
        }
        if (type == Integer.TYPE) {
            return 0;
        }
        if (type == Long.TYPE) {
            return 0L;
        }
        if (type == String.class) {
            return "";
        }
        if (type == ResultSet.class) {
            return resultSet(table(List.of()), 0);
        }
        if (type == Void.TYPE) {
            return null;
        }
        throw unsupported(method);
    }

    static YamlDbTable table(List<Map<String, Object>> rows) {
        Map<String, Boolean> seen = new LinkedHashMap<>();
        for (Map<String, Object> row : rows) {
            for (String key : row.keySet()) {
                seen.putIfAbsent(key, true);
            }
        }
        return new YamlDbTable(new ArrayList<>(seen.keySet()), rows);
    }

    private static Object unwrap(Object proxy, Object iface) throws SQLException {
        Class<?> type = (Class<?>) iface;
        if (type.isInstance(proxy)) {
            return proxy;
        }
        throw new SQLException("Not a wrapper for " + type.getName());
    }

    private static SQLFeatureNotSupportedException unsupported(Method method) {
        return new SQLFeatureNotSupportedException(method.getName() + " is not supported");
    }

    record YamlDbTable(List<String> columns, List<Map<String, Object>> rows) {
    }
}
