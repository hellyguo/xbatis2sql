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

/// 解析过程中数据
pub struct XmlParsedState {
    /// 是否在语句中
    pub in_statement: bool,
    /// 是否在 `sql` 块中
    pub in_sql: bool,
    /// `sql` 索引
    pub sql_idx: i32,
    /// `sql` 临时存储，以索引为键，`sql` 为值
    pub include_temp_sqls: HashMap<i32, String>,
    /// `sql` 临时存储，以 `sql` 的 `id` 为键，索引为值
    pub include_temp_sqls_ids: HashMap<String, i32>,
}

impl XmlParsedState {
    /// 构建器，构造工厂
    pub fn new() -> Self {
        return XmlParsedState {
            in_statement: false,
            in_sql: false,
            sql_idx: 0,
            include_temp_sqls: HashMap::new(),
            include_temp_sqls_ids: HashMap::new(),
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
        for e in parser {
            match e {
                Ok(XmlEvent::StartElement {
                    name, attributes, ..
                }) => {
                    self.parse_start_element(name, attributes, &mut builder, &mut state, sql_store);
                }
                Ok(XmlEvent::EndElement { name }) => {
                    self.parse_end_element(name, &mut builder, &mut state, sql_store);
                }
                Ok(XmlEvent::CData(s)) => {
                    if state.in_statement || state.in_sql {
                        builder.append(s);
                    }
                }
                Ok(XmlEvent::Characters(s)) => {
                    if state.in_statement || state.in_sql {
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
    }

    fn parse_start_element(
        &self,
        name: OwnedName,
        attributes: Vec<OwnedAttribute>,
        builder: &mut StringBuilder,
        state: &mut XmlParsedState,
        sql_store: &mut Vec<String>,
    );

    fn parse_end_element(
        &self,
        name: OwnedName,
        builder: &mut StringBuilder,
        state: &mut XmlParsedState,
        sql_store: &mut Vec<String>,
    ) {
        let element_name = name.local_name.as_str().to_ascii_lowercase();
        if parse_helper::match_statement(&element_name) {
            let sql = parse_helper::replace_included_sql(
                builder,
                &state.include_temp_sqls,
                &state.include_temp_sqls_ids,
            );
            self.clear_and_push(&sql, sql_store);
            state.in_statement = false;
        } else if element_name == "sql" {
            state
                .include_temp_sqls
                .insert(state.sql_idx, builder.to_string());
            state.sql_idx += 1;
            builder.clear();
            state.in_sql = false;
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
