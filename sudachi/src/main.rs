use std::collections::HashMap;
use std::path::Path;
use std::env;
use std::io;
use std::io::Write;
use regex::Regex;
use kanaria::string::{UCSStr, ConvertType};
use kanaria::utils::ConvertTarget;
use lazy_regex::regex_replace_all;
use csv::{ReaderBuilder, Error as CsvError};

use std::result::Result; // 標準ライブラリのResultを明示的にインポート

// カスタムの結果構造体の名前を変更
#[derive(Hash, Eq, PartialEq, Clone)]
struct DictionaryKey {
    yomi: String,
    surface: String,
}

struct DictionaryEntry {
    key: DictionaryKey,
    hinshi_id: i32,
    cost: i32,
}

struct DictionaryData {
    entries: HashMap<DictionaryKey, DictionaryEntry>,
}

impl DictionaryData {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn add(&mut self, entry: DictionaryEntry) {
        self.entries.insert(entry.key.clone(), entry);
    }

    fn output(&self) -> io::Result<()> {
        let mut writer = io::BufWriter::new(io::stdout());
        for entry in self.entries.values() {
            writeln!(
                writer,
                "{}\t{}\t{}\t{}\t{}",
                entry.key.yomi, entry.hinshi_id, entry.hinshi_id, entry.cost, entry.key.surface
            )?;
        }
        writer.flush()
    }
}


type IdDef = HashMap<String, i32>;
type ClassMap = HashMap<String, i32>;
type UserIdDef = HashMap<i32, String>;

const DEFAULT_COST: i32 = 6000;
const MIN_COST: i32 = 0;
const MAX_COST: i32 = 10000;
const COST_ADJUSTMENT: i32 = 10;

mod utils {
    use super::*;

    pub fn convert_to_hiragana(text: &str) -> String {
        let target: Vec<char> = text.chars().collect();
        let mut yomi: String = UCSStr::convert(&target, ConvertType::Hiragana, ConvertTarget::ALL).iter().collect();
        yomi = yomi.replace("ゐ", "い").replace("ゑ", "え");
        yomi
    }

    pub fn unicode_escape_to_char(text: &str) -> String {
        regex_replace_all!(r#"\\u([0-9a-fA-F]{4})"#, text, |_, num: &str| {
            let num: u32 = u32::from_str_radix(num, 16).unwrap();
            std::char::from_u32(num).unwrap().to_string()
        }).to_string()
    }

    pub fn adjust_cost(cost: i32) -> i32 {
        if cost < MIN_COST {
            8000
        } else if cost > MAX_COST {
            MAX_COST
        } else {
            DEFAULT_COST + (cost / COST_ADJUSTMENT)
        }
    }
}

use crate::utils::convert_to_hiragana;
use crate::utils::unicode_escape_to_char;
use crate::utils::adjust_cost;

fn id_expr(clsexpr: &str, id_def: &mut IdDef, class_map: &mut ClassMap) -> i32 {
    let expr = clsexpr.split(',').collect::<Vec<_>>();
    let mut r=-1;
    let mut q=0;
    let mut p;
    if id_def.contains_key(clsexpr) {
        let a = id_def.get(clsexpr);
        match a {
            Some(a) => r = *a,
            None => ()
        }
        if r != -1 {
            class_map.insert(clsexpr.to_string(), r);
            return r;
        }
    } else {
        for h in &mut *id_def {
            p=0;
            for x in expr.iter() {
                if *x == "*" { continue };
                let i = h.0.split(",").collect::<Vec<_>>();
                for y in i {
                    if y == "*"  || y == "自立" || y == "一般" /* || y == "非自立" */ { continue };
                    if *x == y {
                        p = p + 1;
                        continue;
                    };
                };
            };
            if q < p {
                q = p;
                r = *h.1;
            };
        }
        if r == -1 { r = 1847 };
        //if ! id_def.contains_key(clsexpr) {
        id_def.insert(clsexpr.to_string(), r);
        //};
        class_map.insert(clsexpr.to_string(), r);
        return r
    }
    return -1
}

fn read_id_def(path: &Path) -> Result<IdDef, CsvError> {
    let mut hash = IdDef::new();
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b' ')
        .from_path(path)?;

    for result in reader.records() {
        let record = result?;
        let id: i32 = record[0].parse().unwrap();
        let mut expr = record[1].replace("五段・", "五段-")
            .replace("名詞,一般", "名詞,普通名詞")
            .replace("名詞,数,", "名詞,数詞,")
            .replace("形-","形,");
        let mut re = Regex::new(r"五段-カ行[^,]*").unwrap();
        expr = re.replace(&expr, "五段-カ行").to_string();
        re = Regex::new(r"ラ行([^,]*)").unwrap();
        let cap = match re.captures(&expr) {
            Some(i) => i.get(1).unwrap().as_str(),
            None => "",
        };
        if cap != "" {
            let mut s1 = String::from("ラ行,");
            s1.push_str(cap);
            expr = re.replace(&expr, s1).to_string();
        };
        hash.insert(expr, id);
    }
    Ok(hash)
}

fn read_user_id_def(path: &Path) -> Result<UserIdDef, CsvError> {
    let mut hash = UserIdDef::new();
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b" "[0])
        .from_path(path)?;
    for result in reader.records() {
        let data = result?;
        let id = data[0].parse::<i32>().unwrap();
        let hinshi = &data[1];
        hash.insert(id, hinshi.to_string());
    }
    Ok(hash)
}

fn sudachi_read_csv(path: &Path, id_def: &mut HashMap<String, i32>, user_id_def: &HashMap<i32, String>, user_dict_flag: bool, dict_data: &mut DictionaryData) -> Result<(), Box<dyn std::error::Error>> {
    let mut class_map = HashMap::<String, i32>::new();
    let reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b","[0])
        .from_path(path);
    //let mut list = Vec::new();
    let kana_check = Regex::new(r"[ァ-ヺ]").unwrap();
    //let chimei_check = Regex::new(r"地名").unwrap();
    let kigou_check = Regex::new(r"^[a-zA-Z ]+$").unwrap();
    for result in reader?.records() {
        match result {
            Err(_err) => continue,
            Ok(record) => {
                let data = record;
                if &data[11] == "キゴウ" && data[5].contains("記号") { continue };
                if &data[5] == "空白" { continue };
                if kigou_check.is_match(&data[4]) && ! (&data[6] == "固有名詞") { continue };
                if ! kana_check.is_match(&data[11]) { continue };
                if data[7].contains("地名") { continue };
                let mut _yomi: String = convert_to_hiragana(&data[11]);
                let s1 = unicode_escape_to_char(&_yomi);
                let s2 = unicode_escape_to_char(&data[4]);
                let s3 = &data[5].replace("補助記号", "記号"); //.replace("空白","記号");
                let s4 = &data[6].replace(r"^数詞$", "数").replace("非自立可能","非自立");
                let s5 = &data[10].replace("形-", "形,");
                let d: String = format!("{},{},{},{},{},{}", s3, s4, &data[7], &data[8], &data[9], s5);
                let hinshi = class_map.get(&d);
                let hinshi_id;
                if hinshi == None {
                    hinshi_id = id_expr(&d, &mut *id_def, &mut class_map);
                } else {
                    hinshi_id = *hinshi.unwrap();
                }
                let mut cost = data[3].parse::<i32>().unwrap();
                cost = adjust_cost(cost);
                //let class: String = format!("{},{},{},{},{},{},{},{},{}", s1, s2, s3, hinshi_id, &data[6], &data[7], &data[8], &data[9], s4);
                if user_dict_flag {
                    let hinshi = u_search_key(&user_id_def, hinshi_id);
                    if hinshi == "" {
                        dbg!(format!("{}\t{}\t{}\t{}", s1, s2, hinshi, hinshi_id));
                    } else {
                        println!("{}\t{}\t{}\t{}", s1, s2, hinshi, "");
                    }
                } else {
                    dict_data.add(DictionaryEntry {
                        key: DictionaryKey {
                            yomi: s1.to_string(),
                            surface: s2.to_string(),
                        },
                        hinshi_id,
                        cost,
                    });
                }
            }
        }
    }
    Ok(())
}

fn search_key(def: &HashMap::<String, i32>, search: i32) -> String {
    for (key, value) in def {
        if value == &search {
            return key.to_string();
        } else {
            continue;
        }
    }
    return "".to_string();
}

fn u_search_key(def: &HashMap::<i32, String>, search: i32) -> String {
    for (key, value) in def {
        if key == &search {
            return value.to_string();
        } else {
            continue;
        }
    }
    return "".to_string();
}

fn utdict_read_csv(path: &Path, id_def: &mut HashMap<String, i32>, user_id_def: &HashMap<i32, String>, user_dict_flag: bool, dict_data: &mut DictionaryData) -> Result<(), Box<dyn std::error::Error>> {
    let reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b"\t"[0])
        .from_path(path);
    //let mut list = Vec::new();
    let kana_check = Regex::new(r"[ぁ-ゖ]").unwrap();
    let kigou_check = Regex::new(r"^[a-zA-Z ]+$").unwrap();
    for result in reader?.records() {
        match result {
            Err(_err) => continue,
            Ok(record) => {
                let data = record;
                if ! kana_check.is_match(&data[0]) { continue };
                let hinshi_id = data[1].parse::<i32>().unwrap();
                if kigou_check.is_match(&data[0]) && ! search_key(&id_def, hinshi_id).contains("固有名詞") { continue };
                let mut _yomi: String = convert_to_hiragana(&data[0]);
                let s1 = unicode_escape_to_char(&_yomi);
                let s2 = unicode_escape_to_char(&data[4]);
                let mut cost = data[3].parse::<i32>().unwrap();
                cost = adjust_cost(cost);
                //let class: String = format!("{},{},{},{},{},{},{},{},{}", s1, s2, s3, hinshi_id, &data[6], &data[7], &data[8], &data[9], s4);
                if user_dict_flag {
                    let hinshi = u_search_key(&user_id_def, hinshi_id);
                    println!("{}\t{}\t{}\t{}", s1, s2, hinshi, "");
                } else {
                    dict_data.add(DictionaryEntry {
                        key: DictionaryKey {
                            yomi: s1.to_string(),
                            surface: s2.to_string(),
                        },
                        hinshi_id,
                        cost,
                    });
                }
            }
        }
    }
    Ok(())
}

fn neologd_read_csv(path: &Path, id_def: &mut HashMap<String, i32>, user_id_def: &HashMap<i32, String>, user_dict_flag: bool, dict_data: &mut DictionaryData) -> Result<(), Box<dyn std::error::Error>> {
    let mut class_map = HashMap::<String, i32>::new();
    let reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b","[0])
        .from_path(path);
    //let mut list = Vec::new();
    let kana_check = Regex::new(r"[ァ-ヺ]").unwrap();
    //let chimei_check = Regex::new(r"地名").unwrap();
    let kigou_check = Regex::new(r"^[a-zA-Z ]+$").unwrap();
    for result in reader?.records() {
        match result {
            Err(_err) => continue,
            Ok(record) => {
                let data = record;
                if &data[11] == "キゴウ" && data[10].contains("記号") { continue };
                if &data[4] == "空白" { continue };
                if kigou_check.is_match(&data[0]) && ! (&data[5] == "固有名詞") { continue };
                if ! kana_check.is_match(&data[11]) { continue };
                if data[6].contains("地域") { continue };
                let mut _yomi: String = convert_to_hiragana(&data[11]);
                let s1 = unicode_escape_to_char(&_yomi);
                let s2 = unicode_escape_to_char(&data[0]);
                let s3 = &data[4];//.replace("補助記号", "記号"); //.replace("空白","記号");
                let s4;
                if &data[4] == "名詞" && &data[5] == "一般" {
                    s4 = "普通名詞" }
                else {
                    s4 = &data[5]; //.replace("普通名詞", "名詞");
                }
                let s5 = &data[9];//.replace("形-", "形,");
                let d: String = format!("{},{},{},{},{},{}", s3, s4, &data[6], &data[7], &data[8], s5);
                let hinshi = class_map.get(&d);
                let hinshi_id;
                if hinshi == None {
                    hinshi_id = id_expr(&d, &mut *id_def, &mut class_map);
                } else {
                    hinshi_id = *hinshi.unwrap();
                }
                let mut cost = data[3].parse::<i32>().unwrap();
                cost = adjust_cost(cost);
                //let class: String = format!("{},{},{},{},{},{},{},{},{}", s1, s2, s3, hinshi_id, &data[6], &data[7], &data[8], &data[9], s4);
                if user_dict_flag {
                    let hinshi = u_search_key(&user_id_def, hinshi_id);
                    if hinshi == "" {
                        dbg!(format!("{}\t{}\t{}\t{}", s1, s2, hinshi, hinshi_id));
                    } else {
                        println!("{}\t{}\t{}\t{}", s1, s2, hinshi, "");
                    }
                } else {
                    dict_data.add(DictionaryEntry {
                        key: DictionaryKey {
                            yomi: s1.to_string(),
                            surface: s2.to_string(),
                        },
                        hinshi_id,
                        cost,
                    });
                }
            }
        }
    }
    Ok(())
}

fn brief(program: &str) -> String {
    format!(
        "Usage: {} [options]\n\n{}",
        program, "Reads markdown from file or standard input and emits HTML.",
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut dict_data = DictionaryData::new();
    let args: Vec<_> = env::args().collect();
    let mut opts = getopts::Options::new();
    opts.optflag("h", "help", "this help message");
    opts.optopt("f", "csv_file", "csv file", "NAME");
    opts.optopt("i", "id_def", "id_def file path", "NAME");
    opts.optopt("U", "user_id_def", "user_id_def file path", "NAME");
    opts.optflag("s", "sudachi", "Sudachi Dict");
    opts.optflag("n", "neologd", "Neologd Dict");
    opts.optflag("u", "utdict", "UT dict");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => { panic!("{}", f.to_string()) }
    };
    if matches.opt_present("help") {
        println!("{}", opts.usage(&brief(&args[0])));
        return Ok(());
    }

    let mut csv_path: &Path = Path::new("./all.csv");
    let mut id_def_path: &Path = Path::new("../id.def");
    let mut user_id_def_path: &Path = Path::new("../user_dic_id.def");
    let _p1: String;
    let _p2: String;
    let _p3: String;

    if matches.opt_present("csv_file") {
        //csv_path = Path::new(&matches.opt_str("csv_file").unwrap_or("./all.csv".to_string()));
        _p1 = matches.opt_str("f").unwrap_or("./all.csv".to_string());
        //p = String::from(&p);
        csv_path = Path::new(&_p1);
    }
    if matches.opt_present("id_def") {
        _p2 = matches.opt_str("id_def").unwrap_or("../id.def".to_string());
        //p = String::from(&p);
        id_def_path = Path::new(&_p2);
    }
    if matches.opt_present("user_id_def") {
        _p3 = matches.opt_str("user_id_def").unwrap_or("../user_dic_id.def".to_string());
        user_id_def_path = Path::new(&_p3);
    }
    let user_dict_flag = matches.opt_present("user_id_def");
    if matches.opt_present("sudachi") && ! user_dict_flag {
        let mut id_def = read_id_def(&id_def_path)?;
        let user_id_def = HashMap::<i32, String>::new();
        sudachi_read_csv(&csv_path, &mut id_def, &user_id_def, user_dict_flag, &mut dict_data)?;
    } else if matches.opt_present("utdict") && ! user_dict_flag {
        let mut id_def = read_id_def(&id_def_path)?;
        let user_id_def = HashMap::<i32, String>::new();
        utdict_read_csv(&csv_path, &mut id_def, &user_id_def, user_dict_flag, &mut dict_data)?;
    } else if matches.opt_present("neologd") && ! user_dict_flag {
        let mut id_def = read_id_def(&id_def_path)?;
        let user_id_def = HashMap::<i32, String>::new();
        neologd_read_csv(&csv_path, &mut id_def, &user_id_def, user_dict_flag, &mut dict_data)?;
    } else if matches.opt_present("sudachi") && user_dict_flag {
        let mut id_def = read_id_def(&id_def_path)?;
        let user_id_def = read_user_id_def(&user_id_def_path)?;
        sudachi_read_csv(&csv_path, &mut id_def, &user_id_def, user_dict_flag, &mut dict_data)?;
    } else if matches.opt_present("utdict") && user_dict_flag {
        let mut id_def = read_id_def(&id_def_path)?;
        let user_id_def = read_user_id_def(&user_id_def_path)?;
        utdict_read_csv(&csv_path, &mut id_def, &user_id_def, user_dict_flag, &mut dict_data)?;
    } else if matches.opt_present("neologd") && user_dict_flag {
        let mut id_def = read_id_def(&id_def_path)?;
        let user_id_def = read_user_id_def(&user_id_def_path)?;
        neologd_read_csv(&csv_path, &mut id_def, &user_id_def, user_dict_flag, &mut dict_data)?;
    }
    dict_data.output()?;

    Ok(())
}
