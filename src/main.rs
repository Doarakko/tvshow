use chrono::{Duration, Local, Timelike};
use std::collections::BTreeMap;
use std::collections::HashMap;
use structopt::StructOpt;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(StructOpt)]
struct Cli {
    #[structopt(short = "a", long = "area", default_value = "東京")]
    area: String,
    #[structopt(short = "t", long = "hours", default_value = "12")]
    hours: i64,
}

#[derive(Debug)]
#[allow(dead_code)]
struct Program {
    id: String,
    channel: usize,
    name: String,
    description: String,
    link: String,
    start_time: String,
    end_time: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::from_args();

    let area_ids: HashMap<&str, &str> = [
        ("札幌", "1"),
        ("函館", "8"),
        ("旭川", "3"),
        ("帯広", "9"),
        ("釧路", "10"),
        ("北見", "12"),
        ("室蘭", "6"),
        ("青森", "13"),
        ("岩手", "16"),
        ("宮城", "19"),
        ("秋田", "22"),
        ("山形", "25"),
        ("福島", "28"),
        ("東京", "42"),
        ("神奈川", "45"),
        ("埼玉", "37"),
        ("千葉", "40"),
        ("茨城", "31"),
        ("栃木", "33"),
        ("群馬", "35"),
        ("山梨", "50"),
        ("長野", "51"),
        ("新潟", "56"),
        ("愛知", "73"),
        ("石川", "60"),
        ("静岡", "67"),
        ("福井", "62"),
        ("富山", "58"),
        ("三重", "76"),
        ("岐阜", "64"),
        ("大阪", "84"),
        ("京都", "81"),
        ("兵庫", "85"),
        ("和歌山", "93"),
        ("奈良", "91"),
        ("滋賀", "79"),
        ("広島", "101"),
        ("岡山", "98"),
        ("島根", "96"),
        ("鳥取", "95"),
        ("山口", "105"),
        ("愛媛", "112"),
        ("香川", "110"),
        ("徳島", "109"),
        ("高知", "116"),
        ("福岡", "117"),
        ("熊本", "126"),
        ("長崎", "123"),
        ("鹿児島", "131"),
        ("宮崎", "129"),
        ("大分", "127"),
        ("佐賀", "122"),
        ("沖縄", "134"),
        ("北九州", "120"),
    ]
    .iter()
    .cloned()
    .collect();

    let area_id = area_ids.get(&*args.area.to_string());
    if area_id.is_none() {
        let mut s = String::new();
        for k in area_ids.keys() {
            if s.is_empty() {
                s = k.to_string();
            } else {
                s = format!("{} {}", s, k);
            }
        }
        panic!("invalid area, please choose from here.\n{}", s);
    }

    let now = Local::now();
    let now_str = now.format("%Y%m%d%H%M").to_string();
    // TV番組表は5時を起点とするため、5時より前は前日の番組表を取得
    let date = if now.hour() < 5 {
        (now - Duration::days(1)).format("%Y%m%d").to_string()
    } else {
        now.format("%Y%m%d").to_string()
    };

    // 表示終了時刻が5時をまたぐ場合は翌日のデータも必要
    let target_time = now + Duration::hours(args.hours);
    let needs_next_day = if now.hour() < 5 {
        // 現在5時前の場合、表示範囲が5時以降にかかるなら当日のデータも必要
        target_time.hour() >= 5 || target_time.date() > now.date()
    } else {
        // 現在5時以降の場合、表示範囲が翌日5時以降にかかるなら翌日のデータも必要
        target_time.date() > now.date() && target_time.hour() >= 5
    };

    let next_date = if now.hour() < 5 {
        now.format("%Y%m%d").to_string()
    } else {
        (now + Duration::days(1)).format("%Y%m%d").to_string()
    };

    let mut programs = BTreeMap::new();
    let program_selector = scraper::Selector::parse("div #program_area ul li").unwrap();

    // 1日目のデータを取得
    let url = format!(
        "https://bangumi.org/epg/td?broad_cast_date={}&ggm_group_id={}",
        date,
        area_id.unwrap()
    );
    let html = reqwest::get(&url).await?.text().await?;
    let document = scraper::Html::parse_document(&html);
    let channels: Vec<String> = get_channels(&document);
    parse_programs(&document, &program_selector, &mut programs);

    // 2日目のデータを取得（必要な場合）
    if needs_next_day {
        let url2 = format!(
            "https://bangumi.org/epg/td?broad_cast_date={}&ggm_group_id={}",
            next_date,
            area_id.unwrap()
        );
        if let Ok(html2) = reqwest::get(&url2).await {
            if let Ok(text) = html2.text().await {
                let document2 = scraper::Html::parse_document(&text);
                parse_programs(&document2, &program_selector, &mut programs);
            }
        }
    }

    let target = (now + Duration::hours(args.hours))
        .format("%Y%m%d%H%M")
        .to_string();

    // チャンネルごとに番組をグループ化
    let mut channel_programs: HashMap<usize, Vec<&Program>> = HashMap::new();
    for v in programs.values() {
        // 終了済みの番組はスキップ
        if v.end_time < now_str {
            continue;
        }
        // N時間以内に開始する番組のみ表示
        if v.start_time > target {
            continue;
        }
        channel_programs.entry(v.channel).or_default().push(v);
    }

    // 存在するチャンネルをソートして最大8チャンネルまで表示
    let mut sorted_channels: Vec<usize> = channel_programs.keys().cloned().collect();
    sorted_channels.sort();
    sorted_channels.truncate(8);

    // 列幅の設定
    let col_width = 20;
    let time_width = 11;

    // ヘッダー出力（チャンネル名）
    print!("{}", " ".repeat(time_width));
    for &ch in &sorted_channels {
        let ch_name = if ch > 0 && ch <= channels.len() {
            truncate_string(&channels[ch - 1], col_width - 1)
        } else {
            format!("Ch{}", ch)
        };
        print!("│{}", center_string(&ch_name, col_width));
    }
    println!("│");

    // 区切り線
    print!("{}", "─".repeat(time_width));
    for _ in &sorted_channels {
        print!("┼{}", "─".repeat(col_width));
    }
    println!("┤");

    // 現在時刻の時間から表示
    let current_hour = now.hour() as i32;
    let end_hour = current_hour + args.hours as i32;

    for hour in current_hour..=end_hour {
        let display_hour = hour % 24;
        let hour_str = format!("{:02}", display_hour);

        // 各チャンネルのこの時間帯の番組を取得
        let mut hour_programs: Vec<Option<&Program>> = Vec::new();
        for &ch in &sorted_channels {
            if let Some(progs) = channel_programs.get(&ch) {
                let prog = progs
                    .iter()
                    .find(|p| p.start_time.get(8..10) == Some(hour_str.as_str()));
                hour_programs.push(prog.copied());
            } else {
                hour_programs.push(None);
            }
        }

        // 1行目: 時刻と番組名
        print!(" {:02}:00     ", display_hour);
        for prog in &hour_programs {
            if let Some(p) = prog {
                let time_str = format!(
                    "{}:{}",
                    p.start_time.get(8..10).unwrap_or("??"),
                    p.start_time.get(10..12).unwrap_or("??")
                );
                let name = truncate_string(&p.name, col_width - 7);
                let cell = format!("{} {}", time_str, name);
                print!("│{}", pad_string(&cell, col_width));
            } else {
                print!("│{}", " ".repeat(col_width));
            }
        }
        println!("│");

        // 2行目: 番組名の続き
        print!("{}", " ".repeat(time_width));
        for prog in &hour_programs {
            if let Some(p) = prog {
                // 1行目で表示した分をスキップして続きを表示
                let name_chars: Vec<char> = p.name.chars().filter(|c| !is_emoji(*c)).collect();
                let first_line_len = truncate_string(&p.name, col_width - 7).chars().count();
                let remaining: String = name_chars.iter().skip(first_line_len).collect();
                let second_line = truncate_string(&remaining, col_width - 1);
                print!("│ {}", pad_string(&second_line, col_width - 1));
            } else {
                print!("│{}", " ".repeat(col_width));
            }
        }
        println!("│");
    }

    println!("\nThis TV schedule is got from テレビ番組表Gガイド(https://bangumi.org).");

    Ok(())
}

fn get_channels(document: &scraper::Html) -> Vec<String> {
    let mut channels: Vec<String> = Vec::new();
    let channel_selector = scraper::Selector::parse("div #ch_area ul li p").unwrap();
    for node in document.select(&channel_selector) {
        channels.push(node.text().next().unwrap_or("").to_string());
    }

    channels
}

fn parse_programs(
    document: &scraper::Html,
    program_selector: &scraper::Selector,
    programs: &mut BTreeMap<String, Program>,
) {
    for node in document.select(program_selector) {
        let parent = node.parent();
        let channel = parent
            .iter()
            .next()
            .unwrap()
            .value()
            .as_element()
            .unwrap()
            .id()
            .unwrap()
            .to_string();
        let channel_id: usize = channel.replace("program_line_", "").parse().unwrap();

        let id = node.value().attr("se-id");
        let inner_html = node.inner_html();
        let fragment = scraper::Html::parse_fragment(&inner_html);
        let name = get_program_name(&fragment);
        let link = get_program_link(&fragment);
        let description = get_program_description(&fragment);
        let start_time = node.value().attr("s");
        let end_time = node.value().attr("e");

        if name.is_empty() {
            continue;
        }

        let key = start_time.unwrap_or("").to_string() + "_" + &channel_id.to_string();
        // 重複を避ける（同じ番組は上書きしない）
        programs.entry(key).or_insert_with(|| Program {
            id: id
                .unwrap_or("")
                .to_string()
                .get(7..)
                .unwrap_or("")
                .to_string(),
            channel: channel_id,
            name,
            description,
            link,
            start_time: start_time.unwrap_or("").to_string(),
            end_time: end_time.unwrap_or("").to_string(),
        });
    }
}

fn get_program_description(document: &scraper::Html) -> String {
    let selector = scraper::Selector::parse("div p.program_detail").unwrap();
    document
        .select(&selector)
        .next()
        .and_then(|p| p.text().next())
        .unwrap_or("")
        .to_string()
}

fn get_program_link(document: &scraper::Html) -> String {
    let selector = scraper::Selector::parse("div a").unwrap();
    document
        .select(&selector)
        .next()
        .and_then(|p| p.value().attr("href"))
        .unwrap_or("")
        .to_string()
}

fn get_program_name(document: &scraper::Html) -> String {
    let selector = scraper::Selector::parse(".program_title").unwrap();
    document
        .select(&selector)
        .next()
        .and_then(|p| p.text().next())
        .unwrap_or("")
        .to_string()
}

fn is_emoji(c: char) -> bool {
    let code = c as u32;
    // 囲み文字・記号類（🈑🈔など）
    (0x1F100..=0x1F1FF).contains(&code)
        || (0x1F200..=0x1F2FF).contains(&code)
        || (0x1F300..=0x1F9FF).contains(&code)
        || (0x2600..=0x26FF).contains(&code)
        || (0x2700..=0x27BF).contains(&code)
}

fn string_width(s: &str) -> usize {
    let cleaned: String = s.chars().filter(|c| !is_emoji(*c)).collect();
    UnicodeWidthStr::width(cleaned.as_str())
}

fn truncate_string(s: &str, max_width: usize) -> String {
    let cleaned: String = s.chars().filter(|c| !is_emoji(*c)).collect();
    let mut result = String::new();
    let mut width = 0;
    for c in cleaned.chars() {
        let char_width = UnicodeWidthChar::width(c).unwrap_or(0);
        if width + char_width > max_width {
            break;
        }
        result.push(c);
        width += char_width;
    }
    result
}

fn pad_string(s: &str, target_width: usize) -> String {
    let current_width = string_width(s);
    if current_width >= target_width {
        truncate_string(s, target_width)
    } else {
        let padding = target_width - current_width;
        format!("{}{}", s, " ".repeat(padding))
    }
}

fn center_string(s: &str, target_width: usize) -> String {
    let current_width = string_width(s);
    if current_width >= target_width {
        truncate_string(s, target_width)
    } else {
        let total_padding = target_width - current_width;
        let left_padding = total_padding / 2;
        let right_padding = total_padding - left_padding;
        format!(
            "{}{}{}",
            " ".repeat(left_padding),
            s,
            " ".repeat(right_padding)
        )
    }
}
