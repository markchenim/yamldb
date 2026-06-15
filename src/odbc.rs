#![allow(clippy::missing_safety_doc, clippy::manual_dangling_ptr)]

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

struct SqlDbcEntry {
    db: Option<Mutex<YamlDb>>,
}

struct SqlStmtEntry {
    dbc_id: usize,
    results: Vec<Record>,
    current_row: usize,
    columns: Vec<String>,
}

lazy_static::lazy_static! {
    static ref DBC_STORE: Mutex<Vec<Option<SqlDbcEntry>>> = Mutex::new(Vec::new());
    static ref STMT_STORE: Mutex<Vec<Option<SqlStmtEntry>>> = Mutex::new(Vec::new());
}

fn slot_alloc<T>(store: &Mutex<Vec<Option<T>>>, item: T) -> usize {
    let mut v = store.lock().unwrap();
    for (i, slot) in v.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(item);
            return i;
        }
    }
    v.push(Some(item));
    v.len() - 1
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
    let q = query.trim().to_lowercase();
    if !q.starts_with("select") {
        return None;
    }
    if !q.contains("where") {
        return Some(QueryOp::and(vec![]));
    }
    let parts: Vec<&str> = q.splitn(2, "where").collect();
    if parts.len() != 2 {
        return Some(QueryOp::and(vec![]));
    }
    parse_condition(parts[1].trim())
}

fn parse_condition(cond: &str) -> Option<QueryOp> {
    let cond = cond.trim();
    if cond.contains(" and ") {
        let ops: Vec<QueryOp> = cond.split(" and ").filter_map(|p| parse_cmp(p.trim())).collect();
        Some(QueryOp::and(ops))
    } else if cond.contains(" or ") {
        let ops: Vec<QueryOp> = cond.split(" or ").filter_map(|p| parse_cmp(p.trim())).collect();
        Some(QueryOp::or(ops))
    } else {
        parse_cmp(cond)
    }
}

fn parse_cmp(s: &str) -> Option<QueryOp> {
    let s = s.trim();
    for sym in &[">=", "<=", "!=", "=", ">", "<"] {
        if let Some(idx) = s.find(*sym) {
            let key = s[..idx].trim().to_string();
            let val = s[idx + sym.len()..].trim();
            let v = if val.starts_with('\'') && val.ends_with('\'') {
                serde_yaml::Value::String(val[1..val.len() - 1].to_string())
            } else if let Ok(n) = val.parse::<i64>() {
                serde_yaml::Value::Number(n.into())
            } else if let Ok(f) = val.parse::<f64>() {
                serde_yaml::Value::Number(serde_yaml::Number::from(f))
            } else {
                serde_yaml::Value::String(val.to_string())
            };
            return match *sym {
                "=" => Some(QueryOp::eq(key, v)),
                "!=" => Some(QueryOp::ne(key, v)),
                ">" => Some(QueryOp::gt(key, v)),
                "<" => Some(QueryOp::lt(key, v)),
                ">=" => Some(QueryOp::gte(key, v)),
                "<=" => Some(QueryOp::lte(key, v)),
                _ => None,
            };
        }
    }
    None
}

fn val_str(rec: &Record, col: &str) -> String {
    rec.data
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

unsafe fn cstr_to_str<'a>(p: *const c_char) -> Result<&'a str, ()> {
    if p.is_null() {
        return Err(());
    }
    unsafe { CStr::from_ptr(p) }.to_str().map_err(|_| ())
}

unsafe fn write_bytes(dst: *mut c_char, buf_len: c_int, src: &[u8]) {
    unsafe {
        let max = (buf_len - 1) as usize;
        let n = src.len().min(max);
        ptr::copy_nonoverlapping(src.as_ptr(), dst as *mut u8, n);
        *dst.add(n) = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLAllocHandle(
    handle_type: c_int,
    input_handle: *mut c_void,
    output_handle: *mut *mut c_void,
) -> c_int {
    unsafe {
        match handle_type {
            SQL_HANDLE_ENV => {
                *output_handle = std::ptr::dangling_mut::<c_void>();
                SQL_SUCCESS
            }
            SQL_HANDLE_DBC => {
                let id = slot_alloc(&DBC_STORE, SqlDbcEntry { db: None });
                *output_handle = id as *mut c_void;
                SQL_SUCCESS
            }
            SQL_HANDLE_STMT => {
                let dbc_id = input_handle as usize;
                let entry = SqlStmtEntry {
                    dbc_id,
                    results: Vec::new(),
                    current_row: 0,
                    columns: Vec::new(),
                };
                let id = slot_alloc(&STMT_STORE, entry);
                *output_handle = id as *mut c_void;
                SQL_SUCCESS
            }
            _ => SQL_ERROR,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLFreeHandle(handle_type: c_int, handle: *mut c_void) -> c_int {
    let id = handle as usize;
    match handle_type {
        SQL_HANDLE_ENV => SQL_SUCCESS,
        SQL_HANDLE_DBC => {
            let mut v = DBC_STORE.lock().unwrap();
            if id < v.len() {
                v[id] = None;
            }
            SQL_SUCCESS
        }
        SQL_HANDLE_STMT => {
            let mut v = STMT_STORE.lock().unwrap();
            if id < v.len() {
                v[id] = None;
            }
            SQL_SUCCESS
        }
        _ => SQL_ERROR,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLConnect(
    connection_handle: *mut c_void,
    server_name: *const c_char,
    _nl1: c_int,
    _user: *const c_char,
    _nl2: c_int,
    _auth: *const c_char,
    _nl3: c_int,
) -> c_int {
    unsafe {
        let dsn = match cstr_to_str(server_name) {
            Ok(s) => s,
            Err(_) => return SQL_ERROR,
        };
        let path = parse_dsn(dsn).unwrap_or_else(|| dsn.to_string());
        let id = connection_handle as usize;
        let mut v = DBC_STORE.lock().unwrap();
        let entry = match v.get_mut(id).and_then(|e| e.as_mut()) {
            Some(e) => e,
            None => return SQL_ERROR,
        };
        let mut db = YamlDb::new(&path);
        match db.load() {
            Ok(_) => {
                entry.db = Some(Mutex::new(db));
                SQL_SUCCESS
            }
            Err(_) => SQL_ERROR,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLDisconnect(connection_handle: *mut c_void) -> c_int {
    let id = connection_handle as usize;
    let mut v = DBC_STORE.lock().unwrap();
    match v.get_mut(id).and_then(|e| e.as_mut()) {
        Some(entry) => {
            entry.db = None;
            SQL_SUCCESS
        }
        None => SQL_ERROR,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLExecDirect(
    statement_handle: *mut c_void,
    statement_text: *const c_char,
    _text_length: c_int,
) -> c_int {
    unsafe {
        let query = match cstr_to_str(statement_text) {
            Ok(s) => s,
            Err(_) => return SQL_ERROR,
        };

        let stmt_id = statement_handle as usize;

        let query_op = match parse_query(query) {
            Some(op) => op,
            None => return SQL_ERROR,
        };

        let dbc_id = {
            let sv = STMT_STORE.lock().unwrap();
            match sv.get(stmt_id).and_then(|e| e.as_ref()) {
                Some(s) => s.dbc_id,
                None => return SQL_ERROR,
            }
        };

        let records = {
            let dv = DBC_STORE.lock().unwrap();
            let entry = match dv.get(dbc_id).and_then(|e| e.as_ref()) {
                Some(e) => e,
                None => return SQL_ERROR,
            };
            let db = match &entry.db {
                Some(db) => db,
                None => return SQL_ERROR,
            };
            let db = db.lock().unwrap();
            let result = db.query(&query_op);
            result.to_vec().into_iter().cloned().collect::<Vec<Record>>()
        };

        let columns = if let Some(first) = records.first() {
            let mut cols: Vec<String> = first.data.keys().cloned().collect();
            cols.sort();
            cols
        } else {
            Vec::new()
        };

        let mut sv = STMT_STORE.lock().unwrap();
        if let Some(stmt) = sv.get_mut(stmt_id).and_then(|e| e.as_mut()) {
            stmt.results = records;
            stmt.current_row = 0;
            stmt.columns = columns;
            SQL_SUCCESS
        } else {
            SQL_ERROR
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLFetch(statement_handle: *mut c_void) -> c_int {
    let id = statement_handle as usize;
    let mut v = STMT_STORE.lock().unwrap();
    match v.get_mut(id).and_then(|e| e.as_mut()) {
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

#[unsafe(no_mangle)]
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
        let id = statement_handle as usize;
        let v = STMT_STORE.lock().unwrap();
        let stmt = match v.get(id).and_then(|e| e.as_ref()) {
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
        let col = stmt.columns[col_idx].clone();
        let value = val_str(&stmt.results[row_idx], &col);
        let bytes = value.as_bytes();
        write_bytes(target_value, buffer_length, bytes);
        if !strlen_or_ind.is_null() {
            *strlen_or_ind = bytes.len() as c_int;
        }
        SQL_SUCCESS
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLNumResultCols(
    statement_handle: *mut c_void,
    column_count: *mut c_int,
) -> c_int {
    unsafe {
        if column_count.is_null() {
            return SQL_ERROR;
        }
        let id = statement_handle as usize;
        let v = STMT_STORE.lock().unwrap();
        match v.get(id).and_then(|e| e.as_ref()) {
            Some(stmt) => {
                *column_count = stmt.columns.len() as c_int;
                SQL_SUCCESS
            }
            None => SQL_ERROR,
        }
    }
}

#[unsafe(no_mangle)]
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
        let id = statement_handle as usize;
        let v = STMT_STORE.lock().unwrap();
        let stmt = match v.get(id).and_then(|e| e.as_ref()) {
            Some(s) => s,
            None => return SQL_ERROR,
        };
        let col_idx = (column_number - 1) as usize;
        if col_idx >= stmt.columns.len() {
            return SQL_ERROR;
        }
        let name = stmt.columns[col_idx].as_bytes();
        write_bytes(column_name, buffer_length, name);
        if !name_length.is_null() {
            *name_length = name.len() as c_int;
        }
        if !data_type.is_null() {
            *data_type = SQL_VARCHAR;
        }
        SQL_SUCCESS
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SQLRowCount(
    statement_handle: *mut c_void,
    row_count: *mut c_int,
) -> c_int {
    unsafe {
        if row_count.is_null() {
            return SQL_ERROR;
        }
        let id = statement_handle as usize;
        let v = STMT_STORE.lock().unwrap();
        match v.get(id).and_then(|e| e.as_ref()) {
            Some(stmt) => {
                *row_count = stmt.results.len() as c_int;
                SQL_SUCCESS
            }
            None => SQL_ERROR,
        }
    }
}

#[unsafe(no_mangle)]
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
        let id = statement_handle as usize;
        let v = STMT_STORE.lock().unwrap();
        if v.get(id).and_then(|e| e.as_ref()).is_none() {
            return SQL_ERROR;
        }
        if !numeric_attribute.is_null() {
            *numeric_attribute = 256;
        }
        SQL_SUCCESS
    }
}
