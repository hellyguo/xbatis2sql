use super::parse_helper;
use log::*;
use regex::Regex;
use rstring_builder::StringBuilder;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::process;
use std::*;
use xml::attribute::*;
use xml::name::*;
use xml::reader::*;

/// 回车
const CRLF: [u8; 1] = [0x0a];

pub enum Mode {
    Statement,
    Select,
    Insert,
    Update,
    Delete,
    SqlPart,
}

impl Mode {
    pub fn from(name: &str) -> Self {
        match name {
            "statement" => Mode::Statement,
            "select" => Mode::Select,
            "insert" => Mode::Insert,
            "update" => Mode::Update,
            "delete" => Mode::Delete,
            "sql" => Mode::SqlPart,
            _ => panic!("unkown mode"),
        }
    }
}

pub struct SqlStatement {
    pub mode: Mode,
    pub id: String,
    pub sql: String,
}

impl SqlStatement {
    pub fn new(mode: Mode, id: String, sql: String) -> Self {
        return SqlStatement { mode, id, sql };
    }
}

/// 解析过程中数据
pub struct XmlParsedState {
    /// 过程中变化
    /// 是否在语句中
    pub in_statement: bool,
    /// 当前ID
    pub current_id: String,
    /// 过程中累计
    /// 语句集
    pub statements: Vec<SqlStatement>,
    /// 语句集
    pub sql_part_map: HashMap<String, SqlStatement>,
    /// 过程中不再变化
    /// 文件名
    pub filename: String,
}

impl XmlParsedState {
    /// 构建器，构造工厂
    pub fn new() -> Self {
        return XmlParsedState {
            in_statement: false,
            current_id: String::from(""),
            statements: Vec::new(),
            sql_part_map: HashMap::new(),
            filename: String::from(""),
        };
    }
}

/// 解析器
pub trait Parser {
    fn parse(&self, output_dir: &String, files: &Vec<String>) {
        let mut sql_store: Vec<String> = Vec::new();
        for file in files {
            self.check_and_parse(file, &mut sql_store);
        }
        self.save(output_dir, sql_store);
    }

    fn check_and_parse(&self, file: &String, sql_store: &mut Vec<String>) {
        if self.detect_match(file) {
            info!("{:?}", file);
            self.read_and_parse(file, sql_store);
        }
    }

    fn detect_match(&self, file: &String) -> bool;

    fn detect_match_with_regex(&self, file: &String, re: &Regex) -> bool {
        debug!(">>{:?}", file);
        let result = fs::read_to_string(file);
        if result.is_ok() {
            return re.is_match(result.unwrap().as_str());
        } else {
            return false;
        }
    }

    fn read_and_parse(&self, file: &String, sql_store: &mut Vec<String>) {
        self.read_xml(file, sql_store);
    }

    fn read_xml(&self, filename: &String, sql_store: &mut Vec<String>) {
        sql_store.push("-- ".to_string() + filename);
        let file = fs::File::open(filename).unwrap();
        let buf = io::BufReader::new(file);
        let parser = EventReader::new(buf);
        let mut builder = StringBuilder::new();
        let mut state = XmlParsedState::new();
        state.filename = filename.clone();
        for e in parser {
            match e {
                Ok(XmlEvent::StartElement {
                    name, attributes, ..
                }) => {
                    self.parse_start_element(name, attributes, &mut builder, &mut state);
                }
                Ok(XmlEvent::EndElement { name }) => {
                    self.parse_end_element(name, &mut builder, &mut state);
                }
                Ok(XmlEvent::CData(s)) => {
                    if state.in_statement {
                        builder.append(s);
                    }
                }
                Ok(XmlEvent::Characters(s)) => {
                    if state.in_statement {
                        builder.append(s);
                    }
                }
                Err(e) => {
                    info!("Error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        info!("{}", state.filename);
        self.replace_and_fill(sql_store, &state.statements, &state.sql_part_map);
    }

    fn parse_start_element(
        &self,
        name: OwnedName,
        attributes: Vec<OwnedAttribute>,
        builder: &mut StringBuilder,
        state: &mut XmlParsedState,
    );

    fn parse_end_element(
        &self,
        name: OwnedName,
        builder: &mut StringBuilder,
        state: &mut XmlParsedState,
    ) {
        let element_name = name.local_name.as_str().to_ascii_lowercase();
        if parse_helper::match_statement(&element_name) {
            let mode = Mode::from(element_name.as_str());
            match mode {
                Mode::SqlPart => {
                    let sql_stat =
                        SqlStatement::new(mode, state.current_id.clone(), builder.to_string());
                    state
                        .sql_part_map
                        .insert(state.current_id.clone(), sql_stat);
                }
                _ => {
                    let sql_stat =
                        SqlStatement::new(mode, state.current_id.clone(), builder.to_string());
                    state.statements.push(sql_stat);
                }
            }
            state.in_statement = false;
            state.current_id = String::from("");
            builder.clear();
        }
    }

    fn replace_and_fill(
        &self,
        sql_store: &mut Vec<String>,
        statements: &Vec<SqlStatement>,
        sql_part_map: &HashMap<String, SqlStatement>,
    ) {
        for sql in statements {
            sql_store.push("--- ".to_string() + &sql.id);
            let mut sql_stat = sql.sql.clone();
            for sql_part in sql_part_map {
                sql_stat =
                    parse_helper::replace_included_sql(&sql_stat, &sql_part.0, &sql_part.1.sql);
            }
            self.clear_and_push(&sql_stat, sql_store);
        }
    }

    fn clear_and_push(&self, origin_sql: &String, sql_store: &mut Vec<String>);

    fn save(&self, output_dir: &String, sql_store: Vec<String>) {
        info!(
            "write to {:?}/resut.sql, size: {:?}",
            output_dir,
            sql_store.len()
        );
        let r = File::create(output_dir.to_string() + "/result.sql");
        if r.is_err() {
            warn!("try to write sql to {:?} failed", output_dir);
            process::exit(-1);
        }
        let mut f = r.unwrap();
        for sql in sql_store {
            self.write2file(&mut f, &sql.as_bytes(), output_dir);
            self.write2file(&mut f, &CRLF, output_dir);
        }
        self.write2file(&mut f, &CRLF, output_dir);
        let fr = f.flush();
        if fr.is_err() {
            warn!("try to flush file {:?} failed", f);
            process::exit(-1);
        }
    }

    fn write2file(&self, f: &mut File, bdata: &[u8], output_dir: &String) {
        let wr = f.write(bdata);
        if wr.is_err() {
            warn!("try to write [{:?}] to {:?} failed", bdata, output_dir);
            process::exit(-1);
        }
    }
}
