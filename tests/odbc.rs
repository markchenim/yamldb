use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use yamldb::odbc::*;

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn setup_test_yaml(path: &str) {
    let content = r#"- id: user1
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
"#;
    std::fs::write(path, content).unwrap();
}

#[test]
fn test_odbc_connect_and_query() {
    let path = "test_odbc_connect.yaml";
    setup_test_yaml(path);

    unsafe {
        let mut env: *mut c_void = ptr::null_mut();
        let mut dbc: *mut c_void = ptr::null_mut();
        let mut stmt: *mut c_void = ptr::null_mut();

        assert_eq!(
            SQLAllocHandle(SQL_HANDLE_ENV, ptr::null_mut(), &mut env),
            SQL_SUCCESS,
            "alloc env"
        );
        assert_eq!(
            SQLAllocHandle(SQL_HANDLE_DBC, env, &mut dbc),
            SQL_SUCCESS,
            "alloc dbc"
        );
        assert_eq!(
            SQLAllocHandle(SQL_HANDLE_STMT, dbc, &mut stmt),
            SQL_SUCCESS,
            "alloc stmt"
        );

        let dsn = cstr(path);
        let ret = SQLConnect(
            dbc,
            dsn.as_ptr(),
            -1,
            ptr::null(),
            -1,
            ptr::null(),
            -1,
        );
        assert_eq!(ret, SQL_SUCCESS, "SQLConnect should succeed");

        let query = cstr("SELECT * FROM data");
        let ret = SQLExecDirect(stmt, query.as_ptr(), -1);
        assert_eq!(ret, SQL_SUCCESS, "SQLExecDirect should succeed");

        let mut col_count: c_int = 0;
        assert_eq!(SQLNumResultCols(stmt, &mut col_count), SQL_SUCCESS);
        assert_eq!(col_count, 3, "Should have 3 data columns (name, age, city)");

        let mut row_count: c_int = 0;
        assert_eq!(SQLRowCount(stmt, &mut row_count), SQL_SUCCESS);
        assert_eq!(row_count, 3, "Should have 3 rows");

        let mut fetched = 0;
        while SQLFetch(stmt) == SQL_SUCCESS {
            fetched += 1;

            let mut id_buf = [0u8; 64];
            SQLGetData(
                stmt,
                1,
                SQL_CHAR,
                id_buf.as_mut_ptr() as *mut c_char,
                64,
                ptr::null_mut(),
            );
        }
        assert_eq!(fetched, 3, "Should fetch 3 rows");

        SQLDisconnect(dbc);
        SQLFreeHandle(SQL_HANDLE_STMT, stmt);
        SQLFreeHandle(SQL_HANDLE_DBC, dbc);
        SQLFreeHandle(SQL_HANDLE_ENV, env);
    }

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_odbc_where_query() {
    let path = "test_odbc_where.yaml";
    setup_test_yaml(path);

    unsafe {
        let mut env: *mut c_void = ptr::null_mut();
        let mut dbc: *mut c_void = ptr::null_mut();
        let mut stmt: *mut c_void = ptr::null_mut();

        SQLAllocHandle(SQL_HANDLE_ENV, ptr::null_mut(), &mut env);
        SQLAllocHandle(SQL_HANDLE_DBC, env, &mut dbc);
        SQLAllocHandle(SQL_HANDLE_STMT, dbc, &mut stmt);

        let dsn = cstr(path);
        SQLConnect(dbc, dsn.as_ptr(), -1, ptr::null(), -1, ptr::null(), -1);

        let query = cstr("SELECT * FROM data WHERE city = 'Beijing'");
        assert_eq!(SQLExecDirect(stmt, query.as_ptr(), -1), SQL_SUCCESS);

        let mut row_count: c_int = 0;
        SQLRowCount(stmt, &mut row_count);
        assert_eq!(row_count, 2, "Should have 2 Beijing records");

        let mut fetched = 0;
        while SQLFetch(stmt) == SQL_SUCCESS {
            fetched += 1;
        }
        assert_eq!(fetched, 2);

        SQLDisconnect(dbc);
        SQLFreeHandle(SQL_HANDLE_STMT, stmt);
        SQLFreeHandle(SQL_HANDLE_DBC, dbc);
        SQLFreeHandle(SQL_HANDLE_ENV, env);
    }

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_odbc_comparison_query() {
    let path = "test_odbc_cmp.yaml";
    setup_test_yaml(path);

    unsafe {
        let mut env: *mut c_void = ptr::null_mut();
        let mut dbc: *mut c_void = ptr::null_mut();
        let mut stmt: *mut c_void = ptr::null_mut();

        SQLAllocHandle(SQL_HANDLE_ENV, ptr::null_mut(), &mut env);
        SQLAllocHandle(SQL_HANDLE_DBC, env, &mut dbc);
        SQLAllocHandle(SQL_HANDLE_STMT, dbc, &mut stmt);

        let dsn = cstr(path);
        let conn_ret = SQLConnect(dbc, dsn.as_ptr(), -1, ptr::null(), -1, ptr::null(), -1);
        assert_eq!(conn_ret, SQL_SUCCESS, "SQLConnect should succeed for {}", path);

        let query = cstr("SELECT * FROM data WHERE age > 28");
        let exec_ret = SQLExecDirect(stmt, query.as_ptr(), -1);
        assert_eq!(exec_ret, SQL_SUCCESS, "SQLExecDirect should succeed");

        let mut row_count: c_int = 0;
        SQLRowCount(stmt, &mut row_count);
        assert_eq!(row_count, 2, "Should have 2 records with age > 28");

        SQLDisconnect(dbc);
        SQLFreeHandle(SQL_HANDLE_STMT, stmt);
        SQLFreeHandle(SQL_HANDLE_DBC, dbc);
        SQLFreeHandle(SQL_HANDLE_ENV, env);
    }

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_odbc_describe_col() {
    let path = "test_odbc_desc.yaml";
    setup_test_yaml(path);

    unsafe {
        let mut env: *mut c_void = ptr::null_mut();
        let mut dbc: *mut c_void = ptr::null_mut();
        let mut stmt: *mut c_void = ptr::null_mut();

        SQLAllocHandle(SQL_HANDLE_ENV, ptr::null_mut(), &mut env);
        SQLAllocHandle(SQL_HANDLE_DBC, env, &mut dbc);
        SQLAllocHandle(SQL_HANDLE_STMT, dbc, &mut stmt);

        let dsn = cstr(path);
        SQLConnect(dbc, dsn.as_ptr(), -1, ptr::null(), -1, ptr::null(), -1);

        let query = cstr("SELECT * FROM data");
        SQLExecDirect(stmt, query.as_ptr(), -1);

        let mut name_buf = [0u8; 64];
        let mut name_len: c_int = 0;
        let mut data_type: c_int = 0;

        let ret = SQLDescribeCol(
            stmt,
            1,
            name_buf.as_mut_ptr() as *mut c_char,
            64,
            &mut name_len,
            &mut data_type,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        );
        assert_eq!(ret, SQL_SUCCESS);

        let col_name = std::ffi::CStr::from_ptr(name_buf.as_ptr() as *const c_char)
            .to_str()
            .unwrap();
        assert_eq!(data_type, SQL_VARCHAR);
        assert!(!col_name.is_empty(), "Column name should not be empty");

        SQLDisconnect(dbc);
        SQLFreeHandle(SQL_HANDLE_STMT, stmt);
        SQLFreeHandle(SQL_HANDLE_DBC, dbc);
        SQLFreeHandle(SQL_HANDLE_ENV, env);
    }

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_odbc_get_data_values() {
    let path = "test_odbc_values.yaml";
    setup_test_yaml(path);

    unsafe {
        let mut env: *mut c_void = ptr::null_mut();
        let mut dbc: *mut c_void = ptr::null_mut();
        let mut stmt: *mut c_void = ptr::null_mut();

        SQLAllocHandle(SQL_HANDLE_ENV, ptr::null_mut(), &mut env);
        SQLAllocHandle(SQL_HANDLE_DBC, env, &mut dbc);
        SQLAllocHandle(SQL_HANDLE_STMT, dbc, &mut stmt);

        let dsn = cstr(path);
        SQLConnect(dbc, dsn.as_ptr(), -1, ptr::null(), -1, ptr::null(), -1);

        let query = cstr("SELECT * FROM data WHERE name = 'Alice'");
        SQLExecDirect(stmt, query.as_ptr(), -1);

        assert_eq!(SQLFetch(stmt), SQL_SUCCESS);

        let mut col_count: c_int = 0;
        SQLNumResultCols(stmt, &mut col_count);

        let mut values: Vec<String> = Vec::new();
        for col in 1..=col_count {
            let mut buf = [0u8; 256];
            SQLGetData(
                stmt,
                col,
                SQL_CHAR,
                buf.as_mut_ptr() as *mut c_char,
                256,
                ptr::null_mut(),
            );
            let val = std::ffi::CStr::from_ptr(buf.as_ptr() as *const c_char)
                .to_str()
                .unwrap()
                .to_string();
            values.push(val);
        }

        assert!(
            values.iter().any(|v| v == "Alice"),
            "Should contain name Alice"
        );
        assert!(
            values.iter().any(|v| v == "Beijing"),
            "Should contain city Beijing"
        );

        assert_eq!(SQLFetch(stmt), SQL_NO_DATA, "No more rows");

        SQLDisconnect(dbc);
        SQLFreeHandle(SQL_HANDLE_STMT, stmt);
        SQLFreeHandle(SQL_HANDLE_DBC, dbc);
        SQLFreeHandle(SQL_HANDLE_ENV, env);
    }

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_odbc_invalid_handle() {
    unsafe {
        assert_eq!(
            SQLConnect(
                ptr::null_mut(),
                ptr::null(),
                -1,
                ptr::null(),
                -1,
                ptr::null(),
                -1,
            ),
            SQL_ERROR
        );

        assert_eq!(
            SQLExecDirect(ptr::null_mut(), ptr::null(), -1),
            SQL_ERROR
        );

        assert_eq!(SQLFetch(ptr::null_mut()), SQL_ERROR);
        assert_eq!(SQLDisconnect(ptr::null_mut()), SQL_ERROR);
        assert_eq!(SQLFreeHandle(SQL_HANDLE_DBC, ptr::null_mut()), SQL_ERROR);
    }
}
