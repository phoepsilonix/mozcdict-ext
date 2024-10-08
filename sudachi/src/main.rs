use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::env;
use std::io;
use std::io::Write;
use regex::Regex;
use lazy_regex::regex_replace_all;
use csv::{ReaderBuilder, Error as CsvError};
use encoding_rs::UTF_8;
use unicode_normalization::UnicodeNormalization;

use std::result::Result; // 標準ライブラリのResultを明示的にインポート

// 結果構造体
// yomi,surface,hinshi_idの組み合わせで重複チェックされる。
#[derive(Hash, Eq, PartialEq, Clone)]
struct DictionaryKey {
    yomi: String,
    surface: String,
    hinshi_id: i32,
}

// コストと品詞判定で判明した品詞の文字列
struct DictionaryEntry {
    key: DictionaryKey,
    cost: i32,
    pos: String,
}

// システム辞書型式とユーザー辞書型式
struct DictionaryData {
    entries: HashMap<DictionaryKey, DictionaryEntry>,
    user_entries: HashMap<DictionaryKey, DictionaryEntry>,
}

impl DictionaryData {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            user_entries: HashMap::new(),
        }
    }

    fn add(&mut self, entry: DictionaryEntry, is_user_dict: bool) {
        let target = if is_user_dict { &mut self.user_entries } else { &mut self.entries };
        target.insert(entry.key.clone(), entry);
    }

    fn output(&self, user_dict: bool) -> io::Result<()> {
        let mut writer = io::BufWriter::new(io::stdout());

        // システム辞書のエントリーを出力
        if ! user_dict {
            for entry in self.entries.values() {
                writeln!(
                    writer,
                    "{}\t{}\t{}\t{}\t{}",
                    entry.key.yomi, entry.key.hinshi_id, entry.key.hinshi_id, entry.cost, entry.key.surface
                )?;
            }
        } else {
            // -Uオプションが設定されている場合のみユーザー辞書を出力
            for entry in self.user_entries.values() {
                if !self.entries.contains_key(&entry.key) {
                    writeln!(
                        writer,
                        "{}\t{}\t{}\t{}",
                        entry.key.yomi, entry.key.surface, entry.pos, "".to_string()
                    )?;
                }
            }
        }

        writer.flush()
    }
}
// Mozc ソースに含まれるsrc/data/dictionary_oss/id.def
// 更新される可能性がある。
type IdDef = HashMap<String, i32>;

const DEFAULT_COST: i32 = 6000;
const MIN_COST: i32 = 0;
const MAX_COST: i32 = 10000;
const COST_ADJUSTMENT: i32 = 10;

mod utils {
    use super::*;


    // カタカナから読みを平仮名へ
    pub fn convert_to_hiragana(text: &str) -> String {
        let (cow, _, _) = UTF_8.encode(text);
        let decoded = UTF_8.decode(&cow).0;
        let normalized = decoded.nfkc().collect::<String>();
        normalized
            .chars()
            .map(|c| match c {
                'ァ'..='ヶ' => char::from_u32(c as u32 - 0x60).unwrap_or(c),
                'ヷ' => 'わ',
                'ヸ' => 'ゐ',
                'ヹ' => 'ゑ',
                'ヺ' => 'を',
                _ => c,
            })
        .collect::<String>()
            .replace("ゐ", "い")
            .replace("ゑ", "え")
    }

    // Unicode Escapeの記述が含まれる場合、それを変換する。
    pub fn unicode_escape_to_char(text: &str) -> String {
        regex_replace_all!(r#"\\u([0-9a-fA-F]{4})"#, text, |_, num: &str| {
            let num: u32 = u32::from_str_radix(num, 16).unwrap();
            std::char::from_u32(num).unwrap().to_string()
        }).to_string()
    }

    // コスト計算
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

// 辞書データの品詞情報とid.defを比較して品詞のidを確定する。
fn id_expr(clsexpr: &str, id_def: &mut HashMap<String, i32>, class_map: &mut HashMap<String, i32>, default_noun_id: i32) -> i32 {
    if let Some(&r) = id_def.get(clsexpr) {
        class_map.insert(clsexpr.to_string(), r);
        return r;
    }

    let expr: Vec<&str> = clsexpr.split(',').collect();
    let mut best_match = (0, -1); // (マッチ数, ID)

    for (key, &id) in id_def.iter() {
        let key_parts: Vec<&str> = key.split(',').collect();

        // 品詞の主要部分(最初の2-3項目)が一致するかを確認
        if expr.len() >= 2 && key_parts.len() >= 2 &&
            expr[0] == key_parts[0] && expr[1] == key_parts[1] {

                let mut match_count = 2; // 最初の2項目は既に一致している
                let mut is_valid_match = true;

                // 残りの項目をチェック
                for (i, (a, b)) in expr.iter().zip(key_parts.iter()).skip(2).enumerate() {
                    if *b != "*" && *a == *b {
                        match_count += 1;
                    } else if i < 1 { // 3番目の項目（小分類）まで厳密にチェック
                        is_valid_match = false;
                        break;
                    } else {
                        // 4番目以降の項目は部分一致も許容
                        if a.contains(b) || b.contains(a) {
                            match_count += 1;
                        }
                        break; // 最初の不一致で終了
                    }
                }

                // 特殊なケースの処理
                if expr[0] == "名詞" && expr[1] == "固有名詞" {
                    if match_count < 3 { // 固有名詞の場合、より詳細なマッチングを要求
                        is_valid_match = false;
                    }
                } else if expr[0] == "動詞" {
                    // 動詞の活用型のチェック
                    let verb_type = expr.get(4).unwrap_or(&"");
                    if verb_type.contains("五段") && key_parts.iter().any(|&k| k.contains("五段")) {
                        match_count += 1;
                    } else if verb_type.contains("四段") && key_parts.iter().any(|&k| k.contains("四段")) {
                        match_count += 1;
                    } else if verb_type.contains("一段") && key_parts.iter().any(|&k| k.contains("一段")) {
                        match_count += 1;
                    } else if verb_type.contains("カ変") && key_parts.iter().any(|&k| k.contains("カ変")) {
                        match_count += 1;
                    } else if verb_type.contains("サ変") && key_parts.iter().any(|&k| k.contains("サ変")) {
                        match_count += 1;
                    } else if verb_type.contains("ラ変") && key_parts.iter().any(|&k| k.contains("ラ変")) {
                        match_count += 1;
                    }
                }

                if is_valid_match && match_count > best_match.0 {
                    best_match = (match_count, id);
                }
            }
    }

    let result_id = if best_match.1 == -1 { default_noun_id } else { best_match.1 };
    id_def.insert(clsexpr.to_string(), result_id);
    class_map.insert(clsexpr.to_string(), result_id);
    result_id
}

fn read_id_def(path: &Path) -> Result<(IdDef, i32), CsvError> {
    let mut hash = IdDef::new();
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b' ')
        .from_path(path)?;
    let mut default_noun_id: i32 = -1;

    for result in reader.records() {
        let record = result?;
        let id: i32 = record[0].parse().unwrap();
        let mut expr = record[1].replace("名詞,一般", "名詞,普通名詞")
            .replace("名詞,数,", "名詞,数詞,")
            .replace("形-","形,")
            .replace("地域,","地名,");

        // 名詞、一般名詞のIDを保存
        if expr == "名詞,普通名詞,*,*,*,*,*" || expr == "名詞,一般,*,*,*,*,*" {
            default_noun_id = id;
        }

        let mut re = Regex::new(r"五段・カ行[^,]*").unwrap();
        expr = re.replace(&expr, "五段・カ行").to_string();

        re = Regex::new(r"サ変([^,]*)").unwrap();
        let cap = match re.captures(&expr) {
            Some(i) => i.get(1).unwrap().as_str(),
            None => "",
        };
        if cap != "" {
            let mut s1 = String::from("サ変,");
            s1.push_str(cap);
            expr = re.replace(&expr, s1).to_string();
        };

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

        re = Regex::new(r"ワ行([^,]*)").unwrap();
        let cap = match re.captures(&expr) {
            Some(i) => i.get(1).unwrap().as_str(),
            None => "",
        };
        if cap != "" {
            let mut s1 = String::from("ワ行,");
            s1.push_str(cap);
            expr = re.replace(&expr, s1).to_string();
        };

        hash.insert(expr, id);
    }
    Ok((hash, default_noun_id))
}

struct PosMapping {
    user_to_id_def: HashMap<String, Vec<String>>,
    id_def_to_user: HashMap<String, String>,
    id_to_user_pos_cache: HashMap<i32, String>,
}

impl PosMapping {
    fn new() -> Self {
        Self {
            user_to_id_def: HashMap::new(),
            id_def_to_user: HashMap::new(),
            id_to_user_pos_cache: HashMap::new(),
        }
    }

    fn add_mapping(&mut self, user_pos: &str, id_def_pos: &str) {
        self.user_to_id_def.entry(user_pos.to_string())
            .or_insert_with(Vec::new)
            .push(id_def_pos.to_string());
        self.id_def_to_user.insert(id_def_pos.to_string(), user_pos.to_string());
    }
}

fn get_user_pos_by_id(mapping: &mut PosMapping, id_def: &IdDef, hinshi_id: i32) -> Option<String> {
    // キャッシュをチェック
    if let Some(cached_pos) = mapping.id_to_user_pos_cache.get(&hinshi_id) {
        return Some(cached_pos.clone());
    }
    let result = id_def.iter()
        .find(|(_, &id)| id == hinshi_id)
        .and_then(|(pos, _)| {
            let parts: Vec<&str> = pos.split(',').collect();
            let mut best_match: Option<(usize, &String)> = None;

            for (key, value) in &mapping.id_def_to_user {
                let key_parts: Vec<&str> = key.split(',').collect();
                let mut match_count = 0;
                let mut is_valid_match = true;

                // 特殊なケース（記号など）の処理
                if parts[0] == "記号" || parts[0] == "補助記号" {
                    if key_parts[0] == "記号" {
                        return Some(value.clone());
                    }
                    continue;
                }

                // 全項目のマッチングを試みる
                for (i, (a, b)) in parts.iter().zip(key_parts.iter()).enumerate() {
                    if *b != "*" && *a == *b {
                        match_count += 1;
                    } else if i < 2 { // 最初の2項目（品詞大分類、中分類）は必ずマッチする必要がある
                        is_valid_match = false;
                        break;
                    } else {
                        // 後半の項目（活用型など）が一致しない場合
                        // 完全一致でなくても、部分的な一致を許容する
                        if a.contains(b) || b.contains(a) {
                            match_count += 1;
                        }
                    }
                }

                // 固有名詞の場合、より詳細なマッチングを要求
                if parts[0] == "名詞" && parts[1] == "固有名詞" && match_count < 4 {
                    is_valid_match = false;
                }

                // 動詞の活用型のマッチング
                if parts[0] == "動詞" {
                    let verb_type = parts.get(4).unwrap_or(&"");
                    if verb_type.contains("五段") && key_parts.iter().any(|&k| k.contains("五段")) {
                        match_count += 1;
                    } else if verb_type.contains("四段") && key_parts.iter().any(|&k| k.contains("四段")) {
                        match_count += 1;
                    } else if verb_type.contains("一段") && key_parts.iter().any(|&k| k.contains("一段")) {
                        match_count += 1;
                    } else if verb_type.contains("カ変") && key_parts.iter().any(|&k| k.contains("カ変")) {
                        match_count += 1;
                    } else if verb_type.contains("サ変") && key_parts.iter().any(|&k| k.contains("サ変")) {
                        match_count += 1;
                    } else if verb_type.contains("ラ変") && key_parts.iter().any(|&k| k.contains("ラ変")) {
                        match_count += 1;
                    }
                }

                if is_valid_match && (best_match.is_none() || match_count > best_match.unwrap().0) {
                    best_match = Some((match_count, value));
                }
            }

            best_match.map(|(_, v)| v.clone())
        });
    // 結果をキャッシュに保存
    if let Some(ref pos) = result {
        mapping.id_to_user_pos_cache.insert(hinshi_id, pos.clone());
    }

    result
}

fn create_pos_mapping() -> PosMapping {
    let mut mapping = PosMapping::new();

    // ユーザー辞書の品詞とid.defの品詞のマッピングを追加
    mapping.add_mapping("固有名詞", "名詞,固有名詞,一般,*,*,*,*");
    mapping.add_mapping("組織", "名詞,固有名詞,組織,*,*,*,*");
    mapping.add_mapping("地名", "名詞,固有名詞,地名,*,*,*,*");
    mapping.add_mapping("地名", "名詞,固有名詞,国,*,*,*,*");
    mapping.add_mapping("地名", "名詞,接尾,地域,*,*,*,*");
    mapping.add_mapping("名", "名詞,固有名詞,人名,名,*,*,*");
    mapping.add_mapping("姓", "名詞,固有名詞,人名,姓,*,*,*");
    mapping.add_mapping("人名", "名詞,固有名詞,人名,*,*,*,*");
    mapping.add_mapping("接尾人名", "接尾辞,人名,*,*,*,*,*");
    mapping.add_mapping("接尾地名", "接尾辞,地名,*,*,*,*,*");
    mapping.add_mapping("動詞カ行五段", "動詞,一般,*,*,五段・カ行,*,*");
    mapping.add_mapping("動詞カ変", "動詞,一般,*,*,カ変,*,*");
    mapping.add_mapping("動詞サ行五段", "動詞,一般,*,*,五段・サ行,*,*");
    mapping.add_mapping("動詞ハ行四", "動詞,非自立,*,*,四段・ハ行,*,*");
    mapping.add_mapping("動詞マ行五段", "動詞,一般,*,*,五段・マ行,*,*");
    mapping.add_mapping("動詞ラ行五段", "動詞,一般,*,*,五段・ラ行,*,*");
    mapping.add_mapping("動詞ワ行五段", "動詞,自立,*,*,五段・ワ行,*,*");
    mapping.add_mapping("動詞一段", "動詞,一般,*,*,一段,*,*");
    mapping.add_mapping("動詞サ変", "動詞,一般,*,*,サ変,*,*");
    mapping.add_mapping("動詞ラ変", "動詞,自立,*,*,ラ変,*,*");
    mapping.add_mapping("動詞五段", "動詞,一般,*,*,五段,*,*");
    mapping.add_mapping("名詞サ変", "名詞,普通名詞,サ変可能,*,*,*,*");

    mapping.add_mapping("形容詞", "形容詞,一般,*,*,形容詞,*,*");
    mapping.add_mapping("フィラー", "感動詞,フィラー,*,*,*,*,*");
    mapping.add_mapping("BOS/EOS", "BOS/EOS,*,*,*,*,*,*");
    mapping.add_mapping("その他", "その他,*,*,*,*,*,*");
    mapping.add_mapping("感動詞", "感動詞,*,*,*,*,*,*");
    mapping.add_mapping("助詞", "助詞,*,*,*,*,*,*");
    mapping.add_mapping("助動詞", "助動詞,*,*,*,*,*,*");
    mapping.add_mapping("終助詞", "助詞,終助詞,*,*,*,*,*");
    mapping.add_mapping("名詞", "名詞,普通名詞,*,*,*,*,*");
    mapping.add_mapping("固有名詞", "名詞,固有名詞,*,*,*,*,*");
    mapping.add_mapping("数", "名詞,数詞,*,*,*,*,*");
    mapping.add_mapping("助数詞", "名詞,数詞,*,*,*,*,*");
    mapping.add_mapping("接尾一般", "接尾辞,*,*,*,*,*,*");
    mapping.add_mapping("接続詞", "接続詞,*,*,*,*,*,*");
    mapping.add_mapping("接頭語", "接頭辞,*,*,*,*,*,*");
    mapping.add_mapping("副詞", "副詞,*,*,*,*,*,*");
    mapping.add_mapping("形容詞", "形容詞,*,*,*,*,*,*");
    mapping.add_mapping("記号", "補助記号,*,*,*,*,*,*");
    mapping.add_mapping("名詞形動", "形状詞,*,*,*,*,*,*");
    mapping.add_mapping("連体詞", "連体詞,*,*,*,*,*,*");
    mapping.add_mapping("動詞", "動詞,*,*,*,*,*,*");
    mapping.add_mapping("記号", "記号,*,*,*,*,*,*");

    mapping
}

// SudachiDict
fn sudachi_read_csv(path: &Path, id_def: &mut IdDef, dict_data: &mut DictionaryData, default_noun_id: i32, user_dict_flag: bool, chimei_flag: bool, symbol_flag: bool) -> Result<(), csv::Error> {
    let mut class_map = HashMap::<String, i32>::new();
    let mut mapping = create_pos_mapping();
    let reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b","[0])
        .from_path(path);
    //let mut list = Vec::new();
    let kana_check = Regex::new(r"^[ぁ-ゖァ-ヺ]+$").unwrap();
    let eisuu_check = Regex::new(r"^[a-zA-Z0-9]+$").unwrap();
    //let chimei_check = Regex::new(r"地名").unwrap();
    let kigou_check = Regex::new(r"^[a-zA-Z ]+$").unwrap();
    for result in reader?.records() {
        match result {
            Err(_err) => continue,
            Ok(record) => {
                let data = record;
                let s3 = &data[5].replace("補助記号", "記号"); //.replace("空白","記号");
                if ! symbol_flag && &data[11] == "キゴウ" && s3.contains("記号") { continue };
                if ! symbol_flag && s3 == "空白" { continue };
                if ! symbol_flag && kigou_check.is_match(&data[4]) && ! (&data[6] == "固有名詞") { continue };
                if ! kana_check.is_match(&data[11]) { continue };
                // 地名を含む場合、オプション指定がなければ、英数のみの地名だけ残し、それ以外は省く。
                if data[7].contains("地名") {
                    if ! eisuu_check.is_match(&data[0]) && ! chimei_flag { continue };
                };
                let mut _yomi: String = convert_to_hiragana(&data[11]);
                let s1 = unicode_escape_to_char(&_yomi);
                let s2 = unicode_escape_to_char(&data[4]);
                let s4 = &data[6].replace("非自立可能","非自立"); //.replace(r"^数詞$", "数");
                let s5 = &data[9].replace("下一段","一段").replace("一段-","一段,").replace("段-","段・");
                let s6 = &data[10].replace("形-", "形,");
                let d: String = format!("{},{},{},{},{},{}", s3, s4, &data[7], &data[8], s5, s6);
                let hinshi = class_map.get(&d);
                let hinshi_id;
                if hinshi == None {
                    hinshi_id = id_expr(&d, &mut *id_def, &mut class_map, default_noun_id);
                } else {
                    hinshi_id = *hinshi.unwrap();
                }
                let mut cost = data[3].parse::<i32>().unwrap();
                cost = adjust_cost(cost);
                if user_dict_flag {
                    match u_search_key(&mut mapping, id_def, hinshi_id) {
                        Some(hinshi) => {
                            dict_data.add(DictionaryEntry {
                                key: DictionaryKey {
                                    yomi: s1.to_string(),
                                    surface: s2.to_string(),
                                    hinshi_id,
                                },
                                cost,
                                pos: hinshi,
                            }, true);
                        },
                        None => {
                            dict_data.add(DictionaryEntry {
                                key: DictionaryKey {
                                    yomi: s1.to_string(),
                                    surface: s2.to_string(),
                                    hinshi_id,
                                },
                                cost,
                                pos: hinshi_id.to_string(),
                            }, true);
                        }
                    }
                } else {
                    dict_data.add(DictionaryEntry {
                        key: DictionaryKey {
                            yomi: s1.to_string(),
                            surface: s2.to_string(),
                            hinshi_id,
                        },
                        cost,
                        pos: "".to_string(),
                    }, false);
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

fn u_search_key(mapping: &mut PosMapping, id_def: &mut IdDef, hinshi_id: i32) -> Option<String> {
    get_user_pos_by_id(mapping, id_def, hinshi_id)
}

// UtDict
fn utdict_read_csv(path: &Path, id_def: &mut IdDef, dict_data: &mut DictionaryData, user_dict_flag: bool, chimei_flag: bool, symbol_flag: bool) -> Result<(), csv::Error> {
    let mut mapping = create_pos_mapping();
    let reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b"\t"[0])
        .from_path(path);
    //let mut list = Vec::new();
    let kana_check = Regex::new(r"[ぁ-ゖァ-ヺ]").unwrap();
    let kigou_check = Regex::new(r"^[a-zA-Z ]+$").unwrap();
    for result in reader?.records() {
        match result {
            Err(_err) => continue,
            Ok(record) => {
                let data = record;
                if ! kana_check.is_match(&data[0]) { continue };
                let hinshi_id = data[1].parse::<i32>().unwrap();
                if ! symbol_flag && kigou_check.is_match(&data[0]) && ! search_key(&id_def, hinshi_id).contains("固有名詞") { continue };
                if search_key(&id_def, hinshi_id).contains("地名") && ! chimei_flag { continue }
                let mut _yomi: String = convert_to_hiragana(&data[0]);
                let s1 = unicode_escape_to_char(&_yomi);
                let s2 = unicode_escape_to_char(&data[4]);
                let mut cost = data[3].parse::<i32>().unwrap();
                cost = adjust_cost(cost);
                //let class: String = format!("{},{},{},{},{},{},{},{},{}", s1, s2, s3, hinshi_id, &data[6], &data[7], &data[8], &data[9], s4);
                if user_dict_flag {
                    match u_search_key(&mut mapping, id_def, hinshi_id) {
                        Some(hinshi) => {
                            dict_data.add(DictionaryEntry {
                                key: DictionaryKey {
                                    yomi: s1.to_string(),
                                    surface: s2.to_string(),
                                    hinshi_id,
                                },
                                cost,
                                pos: hinshi,
                            }, true);
                        },
                        None => {
                            dict_data.add(DictionaryEntry {
                                key: DictionaryKey {
                                    yomi: s1.to_string(),
                                    surface: s2.to_string(),
                                    hinshi_id,
                                },
                                cost,
                                pos: hinshi_id.to_string(),
                            }, true);
                        }
                    }
                } else {
                    dict_data.add(DictionaryEntry {
                        key: DictionaryKey {
                            yomi: s1.to_string(),
                            surface: s2.to_string(),
                            hinshi_id,
                        },
                        cost,
                        pos: "".to_string(),
                    }, false);
                }
            }
        }
    }
    Ok(())
}

// Neologd
fn neologd_read_csv(path: &Path, id_def: &mut IdDef, dict_data: &mut DictionaryData, default_noun_id: i32, user_dict_flag: bool, chimei_flag: bool, symbol_flag: bool) -> Result<(), csv::Error> {
    let mut mapping = create_pos_mapping();
    let mut class_map = HashMap::<String, i32>::new();
    let reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b","[0])
        .from_path(path);
    //let mut list = Vec::new();
    let kana_check = Regex::new(r"[ぁ-ゖァ-ヺ]").unwrap();
    //let chimei_check = Regex::new(r"地名").unwrap();
    let kigou_check = Regex::new(r"^[a-zA-Z ]+$").unwrap();
    for result in reader?.records() {
        match result {
            Err(_err) => continue,
            Ok(record) => {
                let data = record;
                if &data[11] == "キゴウ" && data[10].contains("記号") { continue };
                if &data[4] == "空白" { continue };
                if ! symbol_flag && kigou_check.is_match(&data[0]) && ! (&data[5] == "固有名詞") { continue };
                if ! kana_check.is_match(&data[11]) { continue };
                if ! chimei_flag && data[6].contains("地域") { continue };
                let mut _yomi: String = convert_to_hiragana(&data[11]);
                let s1 = unicode_escape_to_char(&_yomi);
                let s2 = unicode_escape_to_char(&data[0]);
                let s3 = &data[4];//.replace("補助記号", "記号"); //.replace("空白","記号");
                let s4 = if &data[4] == "名詞" && &data[5] == "一般" {
                    "普通名詞"
                } else if &data[4] == "名詞" && &data[5] == "固有名詞" {
                    &data[5] // 固有名詞はそのまま保持
                } else {
                    &data[5]
                };
                let s5 = &data[9];//.replace("形-", "形,");
                let d: String = format!("{},{},{},{},{},{}", s3, s4, &data[6], &data[7], &data[8], s5);
                let hinshi = class_map.get(&d);
                let hinshi_id;
                if hinshi == None {
                    hinshi_id = id_expr(&d, &mut *id_def, &mut class_map, default_noun_id);
                } else {
                    hinshi_id = *hinshi.unwrap();
                }
                let mut cost = data[3].parse::<i32>().unwrap();
                cost = adjust_cost(cost);
                if user_dict_flag {
                    match u_search_key(&mut mapping, id_def, hinshi_id) {
                        Some(hinshi) => {
                            dict_data.add(DictionaryEntry {
                                key: DictionaryKey {
                                    yomi: s1.to_string(),
                                    surface: s2.to_string(),
                                    hinshi_id,
                                },
                                cost,
                                pos: hinshi,
                            }, true);
                        },
                        None => {
                            dict_data.add(DictionaryEntry {
                                key: DictionaryKey {
                                    yomi: s1.to_string(),
                                    surface: s2.to_string(),
                                    hinshi_id,
                                },
                                cost,
                                pos: hinshi_id.to_string(),
                            }, true);
                        }
                    }
                } else {
                    dict_data.add(DictionaryEntry {
                        key: DictionaryKey {
                            yomi: s1.to_string(),
                            surface: s2.to_string(),
                            hinshi_id,
                        },
                        cost,
                        pos: "".to_string(),
                    }, false);
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
    let args: Vec<_> = env::args().collect();
    let mut dict_data = DictionaryData::new();
    let mut opts = getopts::Options::new();
    opts.optflag("h", "help", "this help message");
    opts.optopt("f", "csv_file", "Dictionary Csv file", "NAME");
    opts.optopt("i", "id_def", "Mozc id.def file path", "NAME");
    opts.optflag("U", "user_dict", "Generate Mozc User Dictionary Formats");
    opts.optflag("s", "sudachi", "Sudachi Dictionary");
    opts.optflag("n", "neologd", "Neologd Dictitonary");
    opts.optflag("u", "utdict", "UT Dictionary");
    opts.optflag("P", "places", "Includes Chimei(Places Name)");
    opts.optflag("S", "Symbols", "Includes Kigou(Symbols)");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => { panic!("{}", f.to_string()) }
    };
    if matches.opt_present("help") {
        println!("{}", opts.usage(&brief(&args[0])));
        return Ok(());
    }

    let current_dir = env::current_dir()?;
    let mut csv_path = current_dir.join("all.csv");
    let mut id_def_path = current_dir.join("id.def");

    if let Some(csv_file) = matches.opt_str("csv_file") {
        csv_path = PathBuf::from(csv_file);
    }

    if let Some(id_def_file) = matches.opt_str("id_def") {
        id_def_path = PathBuf::from(id_def_file);
    }

    let user_dict_flag = matches.opt_present("user_dict");
    let chimei_flag = matches.opt_present("places");
    let symbol_flag = matches.opt_present("Symbols");
    let (mut id_def, default_noun_id) = read_id_def(&id_def_path)?;
    if matches.opt_present("sudachi") && ! user_dict_flag {
        sudachi_read_csv(&csv_path, &mut id_def, &mut dict_data, default_noun_id, user_dict_flag, chimei_flag, symbol_flag)?;
    } else if matches.opt_present("utdict") && ! user_dict_flag {
        utdict_read_csv(&csv_path, &mut id_def, &mut dict_data, user_dict_flag, chimei_flag, symbol_flag)?;
    } else if matches.opt_present("neologd") && ! user_dict_flag {
        neologd_read_csv(&csv_path, &mut id_def, &mut dict_data, default_noun_id, user_dict_flag, chimei_flag, symbol_flag)?;
    } else if matches.opt_present("sudachi") && user_dict_flag {
        sudachi_read_csv(&csv_path, &mut id_def, &mut dict_data, default_noun_id, user_dict_flag, chimei_flag, symbol_flag)?;
    } else if matches.opt_present("utdict") && user_dict_flag {
        utdict_read_csv(&csv_path, &mut id_def, &mut dict_data, user_dict_flag, chimei_flag, symbol_flag)?;
    } else if matches.opt_present("neologd") && user_dict_flag {
        neologd_read_csv(&csv_path, &mut id_def, &mut dict_data, default_noun_id, user_dict_flag, chimei_flag, symbol_flag)?;
    }
    dict_data.output(user_dict_flag)?;

    Ok(())
}
