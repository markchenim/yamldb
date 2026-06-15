use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::Mutex;

use crate::{QueryOp, Record, YamlDb};

pub const SQL_SUCCESS: c_int = 0;
pub const SQL_ERROR: c_int = -1;
pub const SQL_NO_DATA: c_int = 100;
pub const SQL_NTS: c_int = -3;

pub const SQL_HANDLE_ENV: c_int = 1;
pub const SQL_HANDLE_DBC: c_int = 2;
pub const SQL_HANDLE_STMT: c_int = 3;

pub const SQL_FETCH_NEXT: c_int = 1;
pub const SQL_FETCH_FIRST: c_int = 2;

pub const SQL_CHAR: c_int = 1;
pub const SQL_INTEGER: c_int = 4;
pub const SQL_VARCHAR: c_int = 12;

#[repr(C)]
pub struct SqlEnv {
    handle_type: c_int,
}

#[repr(C)]
pub struct SqlDbc {
    handle_type: c_int,
    db: Option<Mutex<YamlDb>>,
}

#[repr(C)]
pub struct SqlStmt {
    handle_type: c_int,
    dbc: *mut SqlDbc,
    results: Vec<Record>,
    current_row: usize,
    columns: Vec<String>,
}

lazy_static::lazy_static! {
    static ref ENVIRONMENTS: Mutex<Vec<Box<SqlEnv>>> = Mutex::new(Vec::new());
    static ref CONNECTIONS: Mutex<Vec<Box<SqlDbc>>> = Mutex::new(Vec::new());
    static ref STATEMENTS: Mutex<Vec<Box<SqlStmt>>> = Mutex::new(Vec::new());
}

fn parse_dsn(dsn: &str) -> Option<String> {
    for part in dsn.split(';') {
        if let Some((key, value)) = part.split_once('=') {
            if key.trim().eq_ignore_ascii_case("DBQ") || key.trim().eq_ignore_ascii_case("FILE") {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn parse_query(query: &str) -> Option<QueryOp> {
    let query = query.trim().to_lowercase();
    
    if query.starts_with("select * from") {
        Some(QueryOp::and(vec![]))
    } else if query.contains("where") {
        let parts: Vec<&str> = query.splitn(2, "where").collect();
        if parts.len() == 2 {
            let condition = parts[1].trim();
            parse_condition(condition)
        } else {
            Some(QueryOp::and(vec![]))
        }
    } else {
        Some(QueryOp::and(vec![]))
    }
}

fn parse_condition(condition: &str) -> Option<QueryOp> {
    let condition = condition.trim();
    
    if condition.contains(" and ") {
        let parts: Vec<&str> = condition.split(" and ").collect();
        let ops: Vec<QueryOp> = parts.iter().filter_map(|p| parse_single_condition(p.trim())).collect();
        Some(QueryOp::and(ops))
    } else if condition.contains(" or ") {
        let parts: Vec<&str> = condition.split(" or ").collect();
        let ops: Vec<QueryOp> = parts.iter().filter_map(|p| parse_single_condition(p.trim())).collect();
        Some(QueryOp::or(ops))
    } else {
        parse_single_condition(condition)
    }
}

fn parse_single_condition(condition: &str) -> Option<QueryOp> {
    let condition = condition.trim();
    
    for op in &[">=", "<=", "!=", "=", ">", "<"] {
        if let Some(idx) = condition.find(op) {
            let key = condition[..idx].trim().to_string();
            let value = condition[idx + op.len()..].trim();
            
            let value = if value.starts_with('\'') && value.ends_with('\'') {
                serde_yaml::Value::String(value[1..value.len()-1].to_string())
            } else if let Ok(n) = value.parse::<i64>() {
                serde_yaml::Value::Number(n.into())
            } else {
                serde_yaml::Value::String(value.to_string())
            };
            
            return match *op {
                "=" => Some(QueryOp::eq(key, value)),
                "!=" => Some(QueryOp::ne(key, value)),
                ">" => Some(QueryOp::gt(key, value)),
                "<" => Some(QueryOp::lt(key, value)),
                ">=" => Some(QueryOp::gte(key, value)),
                "<=" => Some(QueryOp::lte(key, value)),
                _ => None,
            };
        }
    }
    
    None
}

#[no_mangle]
pub unsafe extern "C" fn SQLAllocHandle(
    handle_type: c_int,
    _input_handle: *mut c_void,
    output_handle: *mut *mut c_void,
) -> c_int {
    match handle_type {
        SQL_HANDLE_ENV => {
            let env = Box::new(SqlEnv { handle_type });
            let mut envs = ENVIRONMENTS.lock().unwrap();
            envs.push(env);
            *output_handle = envs.last_mut().unwrap().as_mut() as *mut _ as *mut c_void;
            SQL_SUCCESS
        }
        SQL_HANDLE_DBC => {
            let dbc = Box::new(SqlDbc {
                handle_type,
                db: None,
            });
            let mut conns = CONNECTIONS.lock().unwrap();
            conns.push(dbc);
            *output_handle = conns.last_mut().unwrap().as_mut() as *mut _ as *mut c_void;
            SQL_SUCCESS
        }
        SQL_HANDLE_STMT => {
            let dbc_handle = _input_handle as *mut SqlDbc;
            let stmt = Box::new(SqlStmt {
                handle_type,
                dbc: dbc_handle,
                results: Vec::new(),
                current_row: 0,
                columns: Vec::new(),
            });
            let mut stmts = STATEMENTS.lock().unwrap();
            stmts.push(stmt);
            *output_handle = stmts.last_mut().unwrap().as_mut() as *mut _ as *mut c_void;
            SQL_SUCCESS
        }
        _ => SQL_ERROR,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SQLFreeHandle(
    handle_type: c_int,
    handle: *mut c_void,
) -> c_int {
    match handle_type {
        SQL_HANDLE_ENV => {
            let mut envs = ENVIRONMENTS.lock().unwrap();
            envs.retain(|e| e.as_ref() as *const _ as *const c_void != handle);
            SQL_SUCCESS
        }
        SQL_HANDLE_DBC => {
            let mut conns = CONNECTIONS.lock().unwrap();
            conns.retain(|c| c.as_ref() as *const _ as *const c_void != handle);
            SQL_SUCCESS
        }
        SQL_HANDLE_STMT => {
            let mut stmts = STATEMENTS.lock().unwrap();
            stmts.retain(|s| s.as_ref() as *const _ as *const c_void != handle);
            SQL_SUCCESS
        }
        _ => SQL_ERROR,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SQLConnect(
    connection_handle: *mut SqlDbc,
    server_name: *const c_char,
    _name_length1: c_int,
    _user_name: *const c_char,
    _name_length2: c_int,
    _authentication: *const c_char,
    _name_length3: c_int,
) -> c_int {
    let dsn = match CStr::from_ptr(server_name).to_str() {
        Ok(s) => s,
        Err(_) => return SQL_ERROR,
    };
    
    let path = match parse_dsn(dsn) {
        Some(p) => p,
        None => dsn.to_string(),
    };
    
    let mut db = YamlDb::new(&path);
    match db.load() {
        Ok(_) => {
            (*connection_handle).db = Some(Mutex::new(db));
            SQL_SUCCESS
        }
        Err(_) => SQL_ERROR,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SQLDisconnect(
    connection_handle: *mut SqlDbc,
) -> c_int {
    (*connection_handle).db = None;
    SQL_SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn SQLExecDirect(
    statement_handle: *mut SqlStmt,
    statement_text: *const c_char,
    _text_length: c_int,
) -> c_int {
    let query = match CStr::from_ptr(statement_text).to_str() {
        Ok(s) => s,
        Err(_) => return SQL_ERROR,
    };
    
    let dbc = (*statement_handle).dbc;
    let db = match &(*dbc).db {
        Some(db) => db,
        None => return SQL_ERROR,
    };
    
    let db = db.lock().unwrap();
    let query_op = match parse_query(query) {
        Some(op) => op,
        None => return SQL_ERROR,
    };
    
    let result = db.query(&query_op);
    let records: Vec<Record> = result.to_vec().into_iter().cloned().collect();
    
    let columns = if let Some(first) = records.first() {
        first.data.keys().cloned().collect()
    } else {
        Vec::new()
    };
    
    (*statement_handle).results = records;
    (*statement_handle).current_row = 0;
    (*statement_handle).columns = columns;
    
    SQL_SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn SQLFetch(
    statement_handle: *mut SqlStmt,
) -> c_int {
    let stmt = &mut *statement_handle;
    if stmt.current_row < stmt.results.len() {
        stmt.current_row += 1;
        SQL_SUCCESS
    } else {
        SQL_NO_DATA
    }
}

#[no_mangle]
pub unsafe extern "C" fn SQLGetData(
    statement_handle: *mut SqlStmt,
    column_number: c_int,
    _target_type: c_int,
    target_value: *mut c_char,
    buffer_length: c_int,
    strlen_or_ind: *mut c_int,
) -> c_int {
    let stmt = &*statement_handle;
    let row_idx = stmt.current_row - 1;
    
    if row_idx >= stmt.results.len() || column_number < 1 {
        return SQL_ERROR;
    }
    
    let col_idx = (column_number - 1) as usize;
    if col_idx >= stmt.columns.len() {
        return SQL_ERROR;
    }
    
    let record = &stmt.results[row_idx];
    let col_name = &stmt.columns[col_idx];
    
    let value = record.data.get(col_name)
        .map(|v| match v {
            serde_yaml::Value::String(s) => s.clone(),
            serde_yaml::Value::Number(n) => n.to_string(),
            serde_yaml::Value::Bool(b) => b.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default();
    
    let bytes = value.as_bytes();
    let copy_len = std::cmp::min(bytes.len(), (buffer_length - 1) as usize);
    
    ptr::copy_nonoverlapping(bytes.as_ptr(), target_value as *mut u8, copy_len);
    *target_value.add(copy_len) = 0;
    
    if !strlen_or_ind.is_null() {
        *strlen_or_ind = bytes.len() as c_int;
    }
    
    SQL_SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn SQLNumResultCols(
    statement_handle: *mut SqlStmt,
    column_count: *mut c_int,
) -> c_int {
    *column_count = (*statement_handle).columns.len() as c_int;
    SQL_SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn SQLDescribeCol(
    statement_handle: *mut SqlStmt,
    column_number: c_int,
    column_name: *mut c_char,
    buffer_length: c_int,
    name_length: *mut c_int,
    _data_type: *mut c_int,
    _column_size: *mut c_int,
    _decimal_digits: *mut c_int,
    _nullable: *mut c_int,
) -> c_int {
    let stmt = &*statement_handle;
    let col_idx = (column_number - 1) as usize;
    
    if col_idx >= stmt.columns.len() {
        return SQL_ERROR;
    }
    
    let name = &stmt.columns[col_idx];
    let bytes = name.as_bytes();
    let copy_len = std::cmp::min(bytes.len(), (buffer_length - 1) as usize);
    
    ptr::copy_nonoverlapping(bytes.as_ptr(), column_name as *mut u8, copy_len);
    *column_name.add(copy_len) = 0;
    
    if !name_length.is_null() {
        *name_length = bytes.len() as c_int;
    }
    
    if !_data_type.is_null() {
        *_data_type = SQL_VARCHAR;
    }
    
    SQL_SUCCESS
}
