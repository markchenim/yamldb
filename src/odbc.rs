use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::Mutex;

use crate::{QueryOp, Record, YamlDb};

pub const SQL_SUCCESS: c_int = 0;
pub const SQL_ERROR: c_int = -1;
pub const SQL_NO_DATA: c_int = 100;

pub const SQL_HANDLE_ENV: c_int = 1;
pub const SQL_HANDLE_DBC: c_int = 2;
pub const SQL_HANDLE_STMT: c_int = 3;

pub const SQL_CHAR: c_int = 1;
pub const SQL_INTEGER: c_int = 4;
pub const SQL_VARCHAR: c_int = 12;

const ENV_MAGIC: usize = 0xE001;
const DBC_MAGIC: usize = 0xD001;
const STMT_MAGIC: usize = 0xC001;

struct HandleHeader {
    magic: usize,
}

struct SqlEnvInner {
    header: HandleHeader,
}

struct SqlDbcInner {
    header: HandleHeader,
    db: Option<Mutex<YamlDb>>,
}

struct SqlStmtInner {
    header: HandleHeader,
    dbc_id: usize,
    results: Vec<Record>,
    current_row: usize,
    columns: Vec<String>,
}

lazy_static::lazy_static! {
    static ref CONNECTIONS: Mutex<Vec<Option<SqlDbcInner>>> = Mutex::new(Vec::new());
    static ref STATEMENTS: Mutex<Vec<Option<SqlStmtInner>>> = Mutex::new(Vec::new());
}

fn alloc_dbc(dbc: SqlDbcInner) -> usize {
    let mut conns = CONNECTIONS.lock().unwrap();
    for (i, slot) in conns.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(dbc);
            return i;
        }
    }
    conns.push(Some(dbc));
    conns.len() - 1
}

fn alloc_stmt(stmt: SqlStmtInner) -> usize {
    let mut stmts = STATEMENTS.lock().unwrap();
    for (i, slot) in stmts.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(stmt);
            return i;
        }
    }
    stmts.push(Some(stmt));
    stmts.len() - 1
}

fn get_dbc(handle: *mut c_void) -> Option<&'static mut SqlDbcInner> {
    if handle.is_null() {
        return None;
    }
    let id = handle as usize;
    let mut conns = CONNECTIONS.lock().unwrap();
    if id >= conns.len() {
        return None;
    }
    conns[id].as_mut().filter(|h| h.header.magic == DBC_MAGIC)
}

fn get_stmt(handle: *mut c_void) -> Option<&'static mut SqlStmtInner> {
    if handle.is_null() {
        return None;
    }
    let id = handle as usize;
    let mut stmts = STATEMENTS.lock().unwrap();
    if id >= stmts.len() {
        return None;
    }
    stmts[id].as_mut().filter(|h| h.header.magic == STMT_MAGIC)
}

fn free_dbc(id: usize) {
    let mut conns = CONNECTIONS.lock().unwrap();
    if id < conns.len() {
        conns[id] = None;
    }
}

fn free_stmt(id: usize) {
    let mut stmts = STATEMENTS.lock().unwrap();
    if id < stmts.len() {
        stmts[id] = None;
    }
}

fn parse_dsn(dsn: &str) -> Option<String> {
    for part in dsn.split(';') {
        if let Some((key, value)) = part.split_once('=') {
            let key = key.trim().to_uppercase();
            if key == "DBQ" || key == "FILE" {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn parse_query(query: &str) -> Option<QueryOp> {
    let query = query.trim().to_lowercase();

    if !query.starts_with("select") {
        return None;
    }

    if !query.contains("where") {
        return Some(QueryOp::and(vec![]));
    }

    let parts: Vec<&str> = query.splitn(2, "where").collect();
    if parts.len() != 2 {
        return Some(QueryOp::and(vec![]));
    }

    parse_condition(parts[1].trim())
}

fn parse_condition(condition: &str) -> Option<QueryOp> {
    let condition = condition.trim();

    if condition.contains(" and ") {
        let ops: Vec<QueryOp> = condition
            .split(" and ")
            .filter_map(|p| parse_single_condition(p.trim()))
            .collect();
        Some(QueryOp::and(ops))
    } else if condition.contains(" or ") {
        let ops: Vec<QueryOp> = condition
            .split(" or ")
            .filter_map(|p| parse_single_condition(p.trim()))
            .collect();
        Some(QueryOp::or(ops))
    } else {
        parse_single_condition(condition)
    }
}

fn parse_single_condition(condition: &str) -> Option<QueryOp> {
    let condition = condition.trim();

    for op_sym in &[">=", "<=", "!=", "=", ">", "<"] {
        if let Some(idx) = condition.find(*op_sym) {
            let key = condition[..idx].trim().to_string();
            let value_str = condition[idx + op_sym.len()..].trim();

            let value = if value_str.starts_with('\'') && value_str.ends_with('\'') {
                serde_yaml::Value::String(value_str[1..value_str.len() - 1].to_string())
            } else if let Ok(n) = value_str.parse::<i64>() {
                serde_yaml::Value::Number(n.into())
            } else if let Ok(f) = value_str.parse::<f64>() {
                serde_yaml::Value::Number(serde_yaml::Number::from(f))
            } else {
                serde_yaml::Value::String(value_str.to_string())
            };

            return match *op_sym {
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

fn record_value_string(record: &Record, col: &str) -> String {
    record
        .data
        .get(col)
        .map(|v| match v {
            serde_yaml::Value::String(s) => s.clone(),
            serde_yaml::Value::Number(n) => n.to_string(),
            serde_yaml::Value::Bool(b) => b.to_string(),
            serde_yaml::Value::Null => "NULL".to_string(),
            _ => String::new(),
        })
        .unwrap_or_default()
}

fn write_cstr(dst: *mut c_char, buf_len: c_int, src: &[u8]) {
    let max = (buf_len - 1) as usize;
    let n = src.len().min(max);
    unsafe {
        ptr::copy_nonoverlapping(src.as_ptr(), dst as *mut u8, n);
        *dst.add(n) = 0;
    }
}

#[no_mangle]
pub unsafe extern "C" fn SQLAllocHandle(
    handle_type: c_int,
    _input_handle: *mut c_void,
    output_handle: *mut *mut c_void,
) -> c_int {
    unsafe {
        match handle_type {
            SQL_HANDLE_ENV => {
                *output_handle = (ENV_MAGIC as *mut c_void);
                SQL_SUCCESS
            }
            SQL_HANDLE_DBC => {
                let dbc = SqlDbcInner {
                    header: HandleHeader { magic: DBC_MAGIC },
                    db: None,
                };
                let id = alloc_dbc(dbc);
                *output_handle = id as *mut c_void;
                SQL_SUCCESS
            }
            SQL_HANDLE_STMT => {
                let dbc_id = _input_handle as usize;
                let stmt = SqlStmtInner {
                    header: HandleHeader { magic: STMT_MAGIC },
                    dbc_id,
                    results: Vec::new(),
                    current_row: 0,
                    columns: Vec::new(),
                };
                let id = alloc_stmt(stmt);
                *output_handle = id as *mut c_void;
                SQL_SUCCESS
            }
            _ => SQL_ERROR,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SQLFreeHandle(handle_type: c_int, handle: *mut c_void) -> c_int {
    match handle_type {
        SQL_HANDLE_ENV => SQL_SUCCESS,
        SQL_HANDLE_DBC => {
            free_dbc(handle as usize);
            SQL_SUCCESS
        }
        SQL_HANDLE_STMT => {
            free_stmt(handle as usize);
            SQL_SUCCESS
        }
        _ => SQL_ERROR,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SQLConnect(
    connection_handle: *mut c_void,
    server_name: *const c_char,
    _name_length1: c_int,
    _user_name: *const c_char,
    _name_length2: c_int,
    _authentication: *const c_char,
    _name_length3: c_int,
) -> c_int {
    unsafe {
        if server_name.is_null() {
            return SQL_ERROR;
        }

        let dsn = match CStr::from_ptr(server_name).to_str() {
            Ok(s) => s,
            Err(_) => return SQL_ERROR,
        };

        let path = parse_dsn(dsn).unwrap_or_else(|| dsn.to_string());

        let dbc = match get_dbc(connection_handle) {
            Some(d) => d,
            None => return SQL_ERROR,
        };

        let mut db = YamlDb::new(&path);
        match db.load() {
            Ok(_) => {
                dbc.db = Some(Mutex::new(db));
                SQL_SUCCESS
            }
            Err(_) => SQL_ERROR,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SQLDisconnect(connection_handle: *mut c_void) -> c_int {
    match get_dbc(connection_handle) {
        Some(dbc) => {
            dbc.db = None;
            SQL_SUCCESS
        }
        None => SQL_ERROR,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SQLExecDirect(
    statement_handle: *mut c_void,
    statement_text: *const c_char,
    _text_length: c_int,
) -> c_int {
    unsafe {
        if statement_text.is_null() {
            return SQL_ERROR;
        }

        let query = match CStr::from_ptr(statement_text).to_str() {
            Ok(s) => s,
            Err(_) => return SQL_ERROR,
        };

        let stmt = match get_stmt(statement_handle) {
            Some(s) => s,
            None => return SQL_ERROR,
        };

        let dbc_id = stmt.dbc_id;
        let conns = CONNECTIONS.lock().unwrap();
        let dbc = match conns.get(dbc_id).and_then(|s| s.as_ref()) {
            Some(d) => d,
            None => return SQL_ERROR,
        };

        let db = match &dbc.db {
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
            let mut cols: Vec<String> = first.data.keys().cloned().collect();
            cols.sort();
            cols
        } else {
            Vec::new()
        };

        drop(conns);

        let stmt = match get_stmt(statement_handle) {
            Some(s) => s,
            None => return SQL_ERROR,
        };
        stmt.results = records;
        stmt.current_row = 0;
        stmt.columns = columns;

        SQL_SUCCESS
    }
}

#[no_mangle]
pub unsafe extern "C" fn SQLFetch(statement_handle: *mut c_void) -> c_int {
    match get_stmt(statement_handle) {
        Some(stmt) => {
            if stmt.current_row < stmt.results.len() {
                stmt.current_row += 1;
                SQL_SUCCESS
            } else {
                SQL_NO_DATA
            }
        }
        None => SQL_ERROR,
    }
}

#[no_mangle]
pub unsafe extern "C" fn SQLGetData(
    statement_handle: *mut c_void,
    column_number: c_int,
    _target_type: c_int,
    target_value: *mut c_char,
    buffer_length: c_int,
    strlen_or_ind: *mut c_int,
) -> c_int {
    unsafe {
        if target_value.is_null() || buffer_length <= 0 {
            return SQL_ERROR;
        }

        let stmt = match get_stmt(statement_handle) {
            Some(s) => s,
            None => return SQL_ERROR,
        };

        if stmt.current_row == 0 || stmt.current_row > stmt.results.len() {
            return SQL_ERROR;
        }

        let row_idx = stmt.current_row - 1;
        let col_idx = (column_number - 1) as usize;

        if col_idx >= stmt.columns.len() {
            return SQL_ERROR;
        }

        let col_name = stmt.columns[col_idx].clone();
        let value = record_value_string(&stmt.results[row_idx], &col_name);
        let bytes = value.as_bytes();

        write_cstr(target_value, buffer_length, bytes);

        if !strlen_or_ind.is_null() {
            *strlen_or_ind = bytes.len() as c_int;
        }

        SQL_SUCCESS
    }
}

#[no_mangle]
pub unsafe extern "C" fn SQLNumResultCols(
    statement_handle: *mut c_void,
    column_count: *mut c_int,
) -> c_int {
    unsafe {
        if column_count.is_null() {
            return SQL_ERROR;
        }

        match get_stmt(statement_handle) {
            Some(stmt) => {
                *column_count = stmt.columns.len() as c_int;
                SQL_SUCCESS
            }
            None => SQL_ERROR,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SQLDescribeCol(
    statement_handle: *mut c_void,
    column_number: c_int,
    column_name: *mut c_char,
    buffer_length: c_int,
    name_length: *mut c_int,
    data_type: *mut c_int,
    _column_size: *mut c_int,
    _decimal_digits: *mut c_int,
    _nullable: *mut c_int,
) -> c_int {
    unsafe {
        if column_name.is_null() || buffer_length <= 0 {
            return SQL_ERROR;
        }

        let stmt = match get_stmt(statement_handle) {
            Some(s) => s,
            None => return SQL_ERROR,
        };

        let col_idx = (column_number - 1) as usize;
        if col_idx >= stmt.columns.len() {
            return SQL_ERROR;
        }

        let name = stmt.columns[col_idx].as_bytes();
        write_cstr(column_name, buffer_length, name);

        if !name_length.is_null() {
            *name_length = name.len() as c_int;
        }
        if !data_type.is_null() {
            *data_type = SQL_VARCHAR;
        }

        SQL_SUCCESS
    }
}

#[no_mangle]
pub unsafe extern "C" fn SQLRowCount(
    statement_handle: *mut c_void,
    row_count: *mut c_int,
) -> c_int {
    unsafe {
        if row_count.is_null() {
            return SQL_ERROR;
        }

        match get_stmt(statement_handle) {
            Some(stmt) => {
                *row_count = stmt.results.len() as c_int;
                SQL_SUCCESS
            }
            None => SQL_ERROR,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SQLColAttribute(
    statement_handle: *mut c_void,
    _column_number: c_int,
    _field_identifier: c_int,
    _character_attribute: *mut c_char,
    _buffer_length: c_int,
    _string_length: *mut c_int,
    numeric_attribute: *mut c_int,
) -> c_int {
    unsafe {
        if get_stmt(statement_handle).is_none() {
            return SQL_ERROR;
        }

        if !numeric_attribute.is_null() {
            *numeric_attribute = 256;
        }

        SQL_SUCCESS
    }
}
