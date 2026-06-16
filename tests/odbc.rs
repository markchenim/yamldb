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

fn alloc_handles() -> (*mut c_void, *mut c_void, *mut c_void) {
    let mut env: *mut c_void = ptr::null_mut();
    let mut dbc: *mut c_void = ptr::null_mut();
    let mut stmt: *mut c_void = ptr::null_mut();
    unsafe {
        SQLAllocHandle(SQL_HANDLE_ENV, ptr::null_mut(), &mut env);
        SQLAllocHandle(SQL_HANDLE_DBC, env, &mut dbc);
        SQLAllocHandle(SQL_HANDLE_STMT, dbc, &mut stmt);
    }
    (env, dbc, stmt)
}

fn free_handles(env: *mut c_void, dbc: *mut c_void, stmt: *mut c_void) {
    unsafe {
        SQLFreeHandle(SQL_HANDLE_STMT, stmt);
        SQLFreeHandle(SQL_HANDLE_DBC, dbc);
        SQLFreeHandle(SQL_HANDLE_ENV, env);
    }
}

#[test]
fn test_odbc_driver() {
    // Test 1: Select all
    {
        let path = "test_odbc_all.yaml";
        setup_test_yaml(path);

        unsafe {
            let (env, dbc, stmt) = alloc_handles();

            let dsn = cstr(path);
            assert_eq!(
                SQLConnect(dbc, dsn.as_ptr(), -1, ptr::null(), -1, ptr::null(), -1),
                SQL_SUCCESS,
                "connect"
            );

            let query = cstr("SELECT * FROM data");
            assert_eq!(
                SQLExecDirect(stmt, query.as_ptr(), -1),
                SQL_SUCCESS,
                "exec select all"
            );

            let mut col_count: c_int = 0;
            SQLNumResultCols(stmt, &mut col_count);
            assert_eq!(col_count, 3, "3 data columns");

            let mut row_count: c_int = 0;
            SQLRowCount(stmt, &mut row_count);
            assert_eq!(row_count, 3, "3 rows");

            let mut fetched = 0;
            while SQLFetch(stmt) == SQL_SUCCESS {
                fetched += 1;
            }
            assert_eq!(fetched, 3, "fetch 3 rows");

            SQLDisconnect(dbc);
            free_handles(env, dbc, stmt);
        }
        let _ = std::fs::remove_file(path);
    }

    // Test 2: WHERE string equality
    {
        let path = "test_odbc_where.yaml";
        setup_test_yaml(path);

        unsafe {
            let (env, dbc, stmt) = alloc_handles();

            let dsn = cstr(path);
            SQLConnect(dbc, dsn.as_ptr(), -1, ptr::null(), -1, ptr::null(), -1);

            let query = cstr("SELECT * FROM data WHERE city = 'Beijing'");
            assert_eq!(
                SQLExecDirect(stmt, query.as_ptr(), -1),
                SQL_SUCCESS,
                "exec where"
            );

            let mut row_count: c_int = 0;
            SQLRowCount(stmt, &mut row_count);
            assert_eq!(row_count, 2, "2 Beijing records");

            let mut fetched = 0;
            while SQLFetch(stmt) == SQL_SUCCESS {
                fetched += 1;
            }
            assert_eq!(fetched, 2, "fetch 2 Beijing rows");

            SQLDisconnect(dbc);
            free_handles(env, dbc, stmt);
        }
        let _ = std::fs::remove_file(path);
    }

    // Test 3: WHERE numeric comparison
    {
        let path = "test_odbc_cmp.yaml";
        setup_test_yaml(path);

        unsafe {
            let (env, dbc, stmt) = alloc_handles();

            let dsn = cstr(path);
            assert_eq!(
                SQLConnect(dbc, dsn.as_ptr(), -1, ptr::null(), -1, ptr::null(), -1),
                SQL_SUCCESS,
                "connect for cmp"
            );

            let query = cstr("SELECT * FROM data WHERE age > 28");
            assert_eq!(
                SQLExecDirect(stmt, query.as_ptr(), -1),
                SQL_SUCCESS,
                "exec age > 28"
            );

            let mut row_count: c_int = 0;
            SQLRowCount(stmt, &mut row_count);
            assert_eq!(row_count, 2, "2 records with age > 28");

            SQLDisconnect(dbc);
            free_handles(env, dbc, stmt);
        }
        let _ = std::fs::remove_file(path);
    }

    // Test 4: Describe columns
    {
        let path = "test_odbc_desc.yaml";
        setup_test_yaml(path);

        unsafe {
            let (env, dbc, stmt) = alloc_handles();

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
            assert_eq!(ret, SQL_SUCCESS, "describe col");
            assert_eq!(data_type, SQL_VARCHAR, "column type is VARCHAR");

            let col_name = std::ffi::CStr::from_ptr(name_buf.as_ptr() as *const c_char)
                .to_str()
                .unwrap();
            assert!(!col_name.is_empty(), "column name not empty");

            SQLDisconnect(dbc);
            free_handles(env, dbc, stmt);
        }
        let _ = std::fs::remove_file(path);
    }

    // Test 5: Get data values
    {
        let path = "test_odbc_vals.yaml";
        setup_test_yaml(path);

        unsafe {
            let (env, dbc, stmt) = alloc_handles();

            let dsn = cstr(path);
            SQLConnect(dbc, dsn.as_ptr(), -1, ptr::null(), -1, ptr::null(), -1);

            let query = cstr("SELECT * FROM data WHERE name = 'Alice'");
            SQLExecDirect(stmt, query.as_ptr(), -1);

            assert_eq!(SQLFetch(stmt), SQL_SUCCESS, "fetch Alice");

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

            assert!(values.iter().any(|v| v == "Alice"), "contains Alice");
            assert!(values.iter().any(|v| v == "Beijing"), "contains Beijing");

            assert_eq!(SQLFetch(stmt), SQL_NO_DATA, "no more rows");

            SQLDisconnect(dbc);
            free_handles(env, dbc, stmt);
        }
        let _ = std::fs::remove_file(path);
    }

    // Test 6: Invalid handles
    unsafe {
        assert_eq!(
            SQLConnect(
                ptr::null_mut(),
                ptr::null(),
                -1,
                ptr::null(),
                -1,
                ptr::null(),
                -1
            ),
            SQL_ERROR,
            "null connect"
        );
        assert_eq!(
            SQLExecDirect(ptr::null_mut(), ptr::null(), -1),
            SQL_ERROR,
            "null exec"
        );
        assert_eq!(SQLFetch(ptr::null_mut()), SQL_ERROR, "null fetch");
        assert_eq!(SQLDisconnect(ptr::null_mut()), SQL_ERROR, "null disconnect");
        assert_eq!(
            SQLFreeHandle(SQL_HANDLE_DBC, ptr::null_mut()),
            SQL_ERROR,
            "null free"
        );
    }
}
