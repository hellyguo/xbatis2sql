use super::def::*;
use super::parse_helper::*;
use super::xml_parser::*;
use lazy_static::*;
use regex::Regex;
use xml::attribute::*;
use xml::name::*;

lazy_static! {
    static ref RE: Regex = Regex::new("DTD SQL Map 2\\.0").unwrap();
}

lazy_static! {
    static ref RE_VEC: Vec<RegexReplacement> = create_replcements();
}

/// `iBATIS` 实现
pub const IBATIS_PARSER: IBatisParser = IBatisParser {};

fn create_replcements() -> Vec<RegexReplacement> {
    return vec![
        RegexReplacement::new("[\t ]?--[^\n]*\n", " "),
        RegexReplacement::new("[\r\n\t ]+", " "),
        RegexReplacement::new("\\$\\{[^${]+\\}", "__REPLACE_SCHEMA__"),
        RegexReplacement::new("#[^#]+#", ":?"),
        RegexReplacement::new("\\$[^$]+\\$", ":?"),
        RegexReplacement::new("WHERE[ ]+AND[ ]+", "WHERE "),
        RegexReplacement::new("WHERE[ ]+OR[ ]+", "WHERE "),
        RegexReplacement::new(",[ ]+WHERE", " WHERE"),
        RegexReplacement::new(",$", ""),
    ];
}

pub struct IBatisParser {}

impl Parser for IBatisParser {
    fn detect_match(&self, file: &String) -> bool {
        return self.detect_match_with_regex(file, &RE);
    }

    fn ex_parse_start_element(
        &self,
        _name: OwnedName,
        _element_name: &String,
        attributes: Vec<OwnedAttribute>,
        state: &mut XmlParsedState,
    ) {
        if state.in_statement {
            search_matched_attr(&attributes, "prepend", |attr| {
                state
                    .sql_builder
                    .append(" ")
                    .append(attr.value.as_str())
                    .append(" ");
            });
        }
    }

    fn clear_and_push(&self, sql_store: &mut Vec<String>, origin_sql: &String) {
        self.loop_clear_and_push(sql_store, &RE_VEC, origin_sql)
    }
}
