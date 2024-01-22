use std::path::Path;
use regex::Regex;
use std::collections::HashMap;
use kanaria::string::UCSStr;
use kanaria::utils::ConvertTarget;
use kanaria::string::ConvertType;
use lazy_regex::*;

fn id_expr(clsexpr: &String, id_def: &mut HashMap::<String, i32>, class_map: &mut HashMap::<String, i32>) -> i32 {
  let expr = clsexpr.split(',').collect::<Vec<_>>();
  let mut r=-1;
  let mut q=0;
  let mut p;
  for h in &mut *id_def {
    p=0;
    for x in expr.iter() {
      if *x == "*" { continue };
      let i = h.0.split(",").collect::<Vec<_>>();
      for y in i {
        if y == "*" || y == "自立" || y == "非自立"  || y == "一般"  { continue };
        if *x == y { p = p + 1 };
      };
    };
    if q < p {
      q = p;
      r = *h.1;
    }
  };
  if ! id_def.contains_key(clsexpr) {
      id_def.insert(clsexpr.to_string(), r);
  };
  class_map.insert(clsexpr.to_string(), r);
  return r;
}

fn read_id_def(path: &Path) -> Result<HashMap::<String, i32>, csv::Error> {
  let mut _hash = HashMap::<String, i32>::new();
  let mut reader = csv::ReaderBuilder::new()
      .has_headers(false)
      .delimiter(b" "[0])
      .from_path(path)?;
  for result in reader.records() {
    let data = result?;
    let id = data[0].parse::<i32>().unwrap();
    let mut hinshi = data[1].split(',').collect::<Vec<_>>();
    hinshi.pop();
    let mut expr =  hinshi.join(",");
    expr = expr.replace("五段・", "五段-");
    expr = expr.replace("名詞,一般", "名詞,普通名詞");
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
    expr = expr.replace("形-","形,");
    _hash.insert(expr.to_string(), id);
  }
  Ok(_hash)
}

fn read_csv(path: &Path, id_def: &mut HashMap::<String, i32>) -> Result<(), csv::Error> {
  let mut class_map = HashMap::<String, i32>::new();
  let reader = csv::ReaderBuilder::new()
      .has_headers(false)
      .delimiter(b","[0])
      .from_path(path);
  //let mut list = Vec::new();
  let kana_check = Regex::new(r"[ア-ン]").unwrap();
  let chimei_check = Regex::new(r"地名").unwrap();
  let kigou_check = Regex::new(r"^[a-zA-Z ]+$").unwrap();
  for result in reader?.records() {
    match result {
        Err(_err) => continue,
        Ok(record) => {
    let data = record;
    if &data[11] == "キゴウ" && ( &data[5] == "記号" || &data[5] == "補助記号") { continue };
    if kigou_check.is_match(&data[4]) && &data[6] == "固有名詞" { continue };
    if ! kana_check.is_match(&data[11]) { continue };
    if chimei_check.is_match(&data[7]) { continue };
    let target = &data[11].to_string().chars().collect::<Vec<char>>();
    let mut _yomi: String = UCSStr::convert(target, ConvertType::Hiragana, ConvertTarget::ALL).iter().collect();
    _yomi = _yomi.replace("ゐ", "い");
    _yomi = _yomi.replace("ゑ", "え");
    let s1 = regex_replace_all!(r#"\\u([0-9a-fA-F]{4})"#, &_yomi, |_, num: &str| {
        let num: u32 = u32::from_str_radix(num, 16).unwrap();
        let c: char = std::char::from_u32(num).unwrap();
        c.to_string()
    });
    let s2 = regex_replace_all!(r#"\\u([0-9a-fA-F]{4})"#, &data[4], |_, num: &str| {
        let num: u32 = u32::from_str_radix(num, 16).unwrap();
        let c: char = std::char::from_u32(num).unwrap();
        c.to_string()
    });
    let s3 = &data[5].replace("補助記号", "記号").replace("空白","記号");
    let s4 = &data[6];//.replace("普通名詞", "名詞");
    let s5 = &data[10].replace("形-", "形,");
    let d: String = format!("{},{},{},{},{},{}", s3, s4, &data[7], &data[8], &data[9], s5);
    //let mut hinshi = Some(id_def.get(&Some(&class_map.get(&d))));
    let hinshi = class_map.get(&d);
    let hinshi_id;
    if hinshi == None {
        hinshi_id = id_expr(&d, &mut *id_def, &mut class_map);
    } else {
        hinshi_id = *hinshi.unwrap();
    }
    class_map.insert(d.clone(), hinshi_id);
    let mut cost = data[3].parse::<i32>().unwrap();
    if cost < 0 {
        cost = 8000;
    } else if cost > 10000 {
        cost = 10000;
    } else {
        cost = 6000 + (cost / 10);
    }
    //let class: String = format!("{},{},{},{},{},{},{},{},{}", s1, s2, s3, hinshi_id, &data[6], &data[7], &data[8], &data[9], s4);
    println!("{}\t{}\t{}\t{}\t{}", s1, hinshi_id, hinshi_id, cost, s2);
        }
    }
  }
  Ok(())
}

fn main() -> Result<(), csv::Error> {
  let mut path = Path::new("./id.def");
  let mut id_def = read_id_def(&path)?;
  path = Path::new("./all.csv");

  read_csv(&path, &mut id_def)?;
  Ok(())
}
