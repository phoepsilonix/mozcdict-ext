use std::path::Path;
use regex::Regex;
use std::collections::HashMap;
use kanaria::string::UCSStr;
use kanaria::utils::ConvertTarget;
use kanaria::string::ConvertType;
use lazy_regex::*;
use std::env;


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
  if r == -1 { r = 1847 };
  if ! id_def.contains_key(clsexpr) {
      id_def.insert(clsexpr.to_string(), r);
  };
  class_map.insert(clsexpr.to_string(), r);
  return r;
}

fn read_id_def(path: &Path) -> Result<HashMap::<String, i32>, csv::Error> {
  let mut hash = HashMap::<String, i32>::new();
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
    expr = expr.replace(r"^名詞,一般", "名詞,普通名詞");
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
    hash.insert(expr.to_string(), id);
  }
  Ok(hash)
}

fn read_user_id_def(path: &Path) -> Result<HashMap::<i32, String>, csv::Error> {
  let mut hash = HashMap::<i32, String>::new();
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

fn sudachi_read_csv(path: &Path, id_def: &mut HashMap::<String, i32>, user_id_def: & HashMap::<i32, String>, user_dict_flag: bool) -> Result<(), csv::Error> {
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
            let s3 = &data[5].replace("補助記号", "記号"); //.replace("空白","記号");
            let s4 = &data[6]; //.replace("普通名詞", "名詞");
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
            if cost < 0 {
                cost = 8000;
            } else if cost > 10000 {
                cost = 10000;
            } else {
                cost = 6000 + (cost / 10);
            }
            //let class: String = format!("{},{},{},{},{},{},{},{},{}", s1, s2, s3, hinshi_id, &data[6], &data[7], &data[8], &data[9], s4);
            if user_dict_flag {
                let hinshi = u_search_key(&user_id_def, hinshi_id);
                if hinshi == "" {
                    dbg!(format!("{}\t{}\t{}\t{}", s1, s2, hinshi, hinshi_id));
                } else {
                    println!("{}\t{}\t{}\t{}", s1, s2, hinshi, "");
                }
            } else {
                println!("{}\t{}\t{}\t{}\t{}", s1, hinshi_id, hinshi_id, cost, s2);
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

fn utdict_read_csv(path: &Path, id_def: &mut HashMap::<String, i32>, user_id_def: & HashMap::<i32, String>, user_dict_flag: bool) -> Result<(), csv::Error> {
  let reader = csv::ReaderBuilder::new()
      .has_headers(false)
      .delimiter(b"\t"[0])
      .from_path(path);
  //let mut list = Vec::new();
  let kana_check = Regex::new(r"[ぁ-んゔ]").unwrap();
  let kigou_check = Regex::new(r"^[a-zA-Z ]+$").unwrap();
  for result in reader?.records() {
    match result {
        Err(_err) => continue,
        Ok(record) => {
    let data = record;
    if ! kana_check.is_match(&data[0]) { continue };
    let hinshi_id = data[1].parse::<i32>().unwrap();
    if kigou_check.is_match(&data[0]) && ! search_key(&id_def, hinshi_id).contains("固有名詞") { continue };
    let mut _yomi: String = (&data[0]).to_string();
    _yomi = _yomi.replace("ゐ", "い");
    _yomi = _yomi.replace("ゑ", "え");
    let s1 = regex_replace_all!(r#"\\u([0-9]{4})"#, &_yomi, |_, num: &str| {
        let num: u32 = u32::from_str_radix(num, 16).unwrap();
        let c: char = std::char::from_u32(num).unwrap();
        c.to_string()
    });
    let s2 = regex_replace_all!(r#"\\u([0-9]{4})"#, &data[4], |_, num: &str| {
        let num: u32 = u32::from_str_radix(num, 16).unwrap();
        let c: char = std::char::from_u32(num).unwrap();
        c.to_string()
    });
    let mut cost = data[3].parse::<i32>().unwrap();
    if cost < 0 {
        cost = 8000;
    } else if cost > 10000 {
        cost = 10000;
    } else {
        cost = 6000 + (cost / 10);
    }
    //let class: String = format!("{},{},{},{},{},{},{},{},{}", s1, s2, s3, hinshi_id, &data[6], &data[7], &data[8], &data[9], s4);
    if user_dict_flag {
        let hinshi = u_search_key(&user_id_def, hinshi_id);
        println!("{}\t{}\t{}\t{}", s1, s2, hinshi, "");
    } else {
        println!("{}\t{}\t{}\t{}\t{}", s1, hinshi_id, hinshi_id, cost, s2);
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

fn main() -> Result<(), csv::Error> {
    let args: Vec<_> = env::args().collect();
    let mut opts = getopts::Options::new();
    opts.optflag("h", "help", "this help message");
    opts.optopt("f", "csv_file", "csv file", "NAME");
    opts.optopt("i", "id_def", "id_def file path", "NAME");
    opts.optopt("U", "user_id_def", "user_id_def file path", "NAME");
    opts.optflag("s", "sudachi", "Sudachi Dict");
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
      sudachi_read_csv(&csv_path, &mut id_def, &user_id_def, user_dict_flag)?;
    } else if matches.opt_present("utdict") && ! user_dict_flag {
      let mut id_def = read_id_def(&id_def_path)?;
      let user_id_def = HashMap::<i32, String>::new();
      utdict_read_csv(&csv_path, &mut id_def, &user_id_def, user_dict_flag)?;
    } else if matches.opt_present("sudachi") && user_dict_flag {
      let mut id_def = read_id_def(&id_def_path)?;
      let user_id_def = read_user_id_def(&user_id_def_path)?;
      sudachi_read_csv(&csv_path, &mut id_def, &user_id_def, user_dict_flag)?;
    } else if matches.opt_present("utdict") && user_dict_flag {
      let mut id_def = read_id_def(&id_def_path)?;
      let user_id_def = read_user_id_def(&user_id_def_path)?;
      utdict_read_csv(&csv_path, &mut id_def, &user_id_def, user_dict_flag)?;
    }
    Ok(())
}
