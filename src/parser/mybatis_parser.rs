use super::abt_parser::*;
use super::parse_helper;
use lazy_static::*;
use regex::Regex;
use rstring_builder::StringBuilder;
use xml::attribute::*;
use xml::name::*;

const PARSER: MyBatisParser = MyBatisParser {};

pub fn parse(output_dir: &String, files: &Vec<String>) {
    PARSER.parse(output_dir, files);
}

struct MyBatisParser {}

impl Parser for MyBatisParser {
    fn detect_match(&self, file: &String) -> bool {
        lazy_static! {
            static ref RE: Regex = Regex::new("DTD Mapper 3\\.0").unwrap();
        }
        return self.detect_match_with_regex(file, &RE);
    }

    fn parse_start_element(
        &self,
        name: OwnedName,
        attributes: Vec<OwnedAttribute>,
        builder: &mut StringBuilder,
        state: &mut XmlParsedState,
        sql_store: &mut Vec<String>,
    ) {
        let element_name = name.local_name.as_str().to_ascii_lowercase();
        if parse_helper::match_statement(&element_name) {
            state.in_statement = true;
            parse_helper::search_matched_attr(&attributes, "id", |attr| {
                sql_store.push("-- ".to_string() + attr.value.as_str());
            });
        } else if state.in_statement && element_name == "where" {
            builder.append("where ");
        } else if state.in_statement && element_name == "set" {
            builder.append("set ");
        } else if state.in_statement && element_name == "include" {
            parse_helper::search_matched_attr(&attributes, "refid", |attr| {
                builder.append("__INCLUDE_ID_");
                builder.append(attr.value.as_str());
                builder.append("_END__");
            });
        } else if element_name == "sql" {
            state.in_sql = true;
            parse_helper::search_matched_attr(&attributes, "id", |attr| {
                state
                    .include_temp_sqls_ids
                    .insert(attr.value.as_str().to_string(), state.sql_idx);
            });
        }
    }

    fn clear_and_push(&self, origin_sql: &String, sql_store: &mut Vec<String>) {
        lazy_static! {
            static ref RE0: Regex = Regex::new("[\r\n\t ]+").unwrap();
            static ref RE1: Regex = Regex::new("#\\{[^#{]+\\}").unwrap();
            static ref RE2: Regex = Regex::new("\\$\\{[^${]+\\}").unwrap();
            static ref RE_FIX1: Regex = Regex::new("WHERE[ ]+AND").unwrap();
            static ref RE_FIX2: Regex = Regex::new("WHERE[ ]+OR").unwrap();
        }
        let mut sql = String::from(origin_sql);
        sql = RE0.replace_all(sql.as_str(), " ").to_string();
        sql = RE1.replace_all(sql.as_str(), ":?").to_string();
        sql = RE2.replace_all(sql.as_str(), ":?").to_string();
        sql = RE_FIX1.replace_all(sql.as_str(), "WHERE").to_string();
        sql = RE_FIX2.replace_all(sql.as_str(), "WHERE").to_string();
        sql_store.push(sql + ";");
    }
}
