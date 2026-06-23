package io.github.markchenim.yamldb.jdbc;

import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.Connection;
import java.sql.DatabaseMetaData;
import java.sql.DriverManager;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.Statement;

public final class YamlDbJdbcDriverTest {
    public static void main(String[] args) throws Exception {
        Class.forName("io.github.markchenim.yamldb.jdbc.YamlDbJdbcDriver");

        Path db = Files.createTempFile("yamldb-jdbc-", ".yaml");
        Files.writeString(db, """
                - id: user1
                  name: "Alice: Smith"
                  age: 30
                  city: Beijing
                  tags:
                    - admin
                    - ops
                - id: user2
                  name: Bob
                  age: 25
                  city: Shanghai
                - id: user3
                  name: Charlie
                  age: 35
                  city: Beijing
                """);

        try (Connection connection = DriverManager.getConnection("jdbc:yamldb:" + db);
             Statement statement = connection.createStatement();
             ResultSet resultSet = statement.executeQuery("SELECT * FROM data WHERE city = 'Beijing' AND age >= 30")) {

            ResultSetMetaData metaData = resultSet.getMetaData();
            assertEquals(5, metaData.getColumnCount(), "column count");

            int rows = 0;
            while (resultSet.next()) {
                rows++;
                String city = resultSet.getString("city");
                int age = resultSet.getInt("age");
                if (!"Beijing".equals(city) || age < 30) {
                    throw new AssertionError("unexpected row: " + city + ", " + age);
                }
                if ("user1".equals(resultSet.getString("id"))) {
                    assertEquals("Alice: Smith", resultSet.getString("name"), "quoted scalar");
                    assertEquals("[\"admin\",\"ops\"]", resultSet.getString("tags"), "nested array");
                }
            }
            assertEquals(2, rows, "row count");
        } finally {
            Files.deleteIfExists(db);
        }

        Path dir = Files.createTempDirectory("yamldb-jdbc-dir-");
        Path users = dir.resolve("users.yaml");
        Path teams = dir.resolve("teams.yml");
        Files.writeString(users, """
                - id: user1
                  name: Alice
                  age: 30
                - id: user2
                  name: Bob
                  age: 25
                """);
        Files.writeString(teams, """
                - id: team1
                  title: Platform
                """);

        try (Connection connection = DriverManager.getConnection("jdbc:yamldb:" + dir);
             Statement statement = connection.createStatement();
             ResultSet resultSet = statement.executeQuery("SELECT * FROM users WHERE age >= 30")) {
            assertEquals(true, resultSet.next(), "directory query row");
            assertEquals("Alice", resultSet.getString("name"), "directory query value");
            assertEquals(false, resultSet.next(), "directory query end");

            DatabaseMetaData metaData = connection.getMetaData();
            int tables = 0;
            try (ResultSet tableRows = metaData.getTables(null, null, "%", null)) {
                while (tableRows.next()) {
                    String table = tableRows.getString("TABLE_NAME");
                    if ("users".equals(table) || "teams".equals(table)) {
                        tables++;
                    }
                }
            }
            assertEquals(2, tables, "directory metadata tables");

            int columns = 0;
            try (ResultSet columnRows = metaData.getColumns(null, null, "users", "%")) {
                while (columnRows.next()) {
                    if ("age".equals(columnRows.getString("COLUMN_NAME"))
                            || "name".equals(columnRows.getString("COLUMN_NAME"))) {
                        columns++;
                    }
                }
            }
            assertEquals(2, columns, "directory metadata columns");
        } finally {
            Files.deleteIfExists(users);
            Files.deleteIfExists(teams);
            Files.deleteIfExists(dir);
        }
    }

    private static void assertEquals(int expected, int actual, String label) {
        if (expected != actual) {
            throw new AssertionError(label + ": expected " + expected + ", got " + actual);
        }
    }

    private static void assertEquals(String expected, String actual, String label) {
        if (!expected.equals(actual)) {
            throw new AssertionError(label + ": expected " + expected + ", got " + actual);
        }
    }

    private static void assertEquals(boolean expected, boolean actual, String label) {
        if (expected != actual) {
            throw new AssertionError(label + ": expected " + expected + ", got " + actual);
        }
    }
}
