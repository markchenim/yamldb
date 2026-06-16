package io.github.markchenim.yamldb.jdbc;

import java.sql.Connection;
import java.sql.Driver;
import java.sql.DriverManager;
import java.sql.DriverPropertyInfo;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.util.Properties;
import java.util.logging.Logger;

public final class YamlDbJdbcDriver implements Driver {
    public static final String URL_PREFIX = "jdbc:yamldb:";

    static {
        try {
            DriverManager.registerDriver(new YamlDbJdbcDriver());
        } catch (SQLException e) {
            throw new ExceptionInInitializerError(e);
        }
    }

    @Override
    public Connection connect(String url, Properties info) throws SQLException {
        if (!acceptsURL(url)) {
            return null;
        }
        return YamlDbJdbcProxies.connection(pathFromUrl(url));
    }

    @Override
    public boolean acceptsURL(String url) {
        return url != null && url.startsWith(URL_PREFIX) && url.length() > URL_PREFIX.length();
    }

    @Override
    public DriverPropertyInfo[] getPropertyInfo(String url, Properties info) {
        return new DriverPropertyInfo[0];
    }

    @Override
    public int getMajorVersion() {
        return 0;
    }

    @Override
    public int getMinorVersion() {
        return 7;
    }

    @Override
    public boolean jdbcCompliant() {
        return false;
    }

    @Override
    public Logger getParentLogger() throws SQLFeatureNotSupportedException {
        throw new SQLFeatureNotSupportedException("YamlDB JDBC does not use java.util.logging");
    }

    private static String pathFromUrl(String url) {
        String path = url.substring(URL_PREFIX.length());
        if (path.startsWith("file:")) {
            return path.substring("file:".length());
        }
        return path;
    }
}
