package io.github.markchenim.yamldb.jdbc;

import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.Connection;
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
                  name: Alice
                  age: 30
                  city: Beijing
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
            assertEquals(4, metaData.getColumnCount(), "column count");

            int rows = 0;
            while (resultSet.next()) {
                rows++;
                String city = resultSet.getString("city");
                int age = resultSet.getInt("age");
                if (!"Beijing".equals(city) || age < 30) {
                    throw new AssertionError("unexpected row: " + city + ", " + age);
                }
            }
            assertEquals(2, rows, "row count");
        } finally {
            Files.deleteIfExists(db);
        }
    }

    private static void assertEquals(int expected, int actual, String label) {
        if (expected != actual) {
            throw new AssertionError(label + ": expected " + expected + ", got " + actual);
        }
    }
}
