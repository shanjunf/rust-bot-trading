// #[path = "../common.rs"]
mod common;

// #[path = "../constant.rs"]
mod constant;

// #[path = "../db/mod.rs"]
mod db;


use mongodb::{bson:: {doc, oid::ObjectId}, options::{FindOneOptions, FindOptions}};

use db::{ Operator, Candle, Macd, Zigzag, Trend, MacdVergence, Order};
use common::{ Result, CustomError };
use std::{collections::{ HashMap, HashSet }, vec};
use std::fmt::format;

const DEPTH:i32 = 12;

macro_rules! get {
    ($var:expr, $field:ident) => {
        $var.$field
    };
}

fn first_ema <const range: usize>(arr: &[f64]) -> f64 {
    if arr.len() < range {
        return 0.0;
    }
    avg(&arr[0..range])
}

fn next_ema(prev_ema: f64, value: f64, range: usize) -> f64 {
    let c = smooth(range);
    toFixed(c * value + ((1.0-c)*prev_ema))
}

fn smooth(n: usize) -> f64{
    2.0 / (n as f64 + 1.0)
}

fn toFixed(n: f64) -> f64{
    let ret = format!("{:.8}", n);
    let r = match ret.parse::<f64>() {
        Ok(x) => x,
        Err(e) => {
            println!("{:?}", e);
            0.0
        }
    };

    r
}

fn avg(arr: &[f64]) -> f64 {
    let mut total = 0.0;
    for val in arr {
        total += val;
    }

    total / arr.len() as f64

}

// 34
fn first_macd(arr: &[f64]) -> (f64, f64, f64, f64, f64) {
    let def = (0.0,0.0,0.0,0.0,0.0);
    if arr.len() < 34 {
        return def;
    }

    const N9: usize = 9;
    const N12: usize = 12;
    const N26: usize = 26;

    let arr12:&[f64] = &arr[0..N12];
    let mut ema12 = first_ema::<N12>(arr12);

    for i in N12..N26 {
        ema12 = next_ema(ema12, arr[i], N12);
    }
    
    let arr26:&[f64] = &arr[0..N26];
    let mut ema26 = first_ema::<N26>(arr26);

    let mut diffs: Vec<f64> = Vec::new();
    diffs.push(ema12 - ema26);

    for i in N26..34 {
        ema12 = next_ema(ema12, arr[i], N12);
        ema26 = next_ema(ema26, arr[i], N26);

        diffs.push(ema12 - ema26);
    }

    let dif = diffs[N9-1];

    let dea = first_ema::<N9>(&diffs[0..diffs.len()]);

    let macd = dif - dea;

    (dif, dea, macd, ema12, ema26)
}

fn next_macd(prev_dea: f64, prev_ema12: f64, prev_ema26: f64, value: f64) -> (f64, f64, f64, f64, f64) {
    let ema12 = next_ema(prev_ema12, value, 12);
    let ema26 = next_ema(prev_ema26, value, 26);
    let diff = ema12 - ema26;

    let dea = next_ema(prev_dea, diff, 9);
    let macd = diff - dea;

    (diff, dea, macd, ema12, ema26)
}

fn proc_ema(symbol: &str, interval: &str, range: usize) -> Result<()> {

    if range != 10 && range != 20 {
        let err = Box::new(CustomError("error: range must equal 10 or 20".to_string()));
        return Err(err);
    }

    let ema_key = format!("ema{}", range);
    let mut time = 0_i64;
    let ret = Candle::collection().find_one(doc!{"symbol": symbol, "interval" : interval, ema_key.as_str(): {"$exists": true}}, FindOneOptions::builder().sort(doc!{ "time": -1 }).build())?;

    if let Some(candle) = ret {
        time = candle.time;
        // get!(candle, ema_key);

    }
    else {
        let cursor = Candle::collection().find(doc!{"symbol": symbol, "interval" : interval}, FindOptions::builder().sort(doc!{"time": 1}).limit(range as i64).build())?;
        
        let mut index = 0;
        let mut vec:Vec<f64> = Vec::new();
        let mut last_id = None;
        for r in cursor {
            index += 1;

            let candle = r?;

            vec.push(candle.close);
            last_id = Some(candle.id);
            time = candle.time;
        }

        let mut ema = 0.0;
        if range == 10 {
            const N:usize = 10;
            ema = first_ema::<N>(&vec[0..N]);
        }
        else if range == 20 {
            const N:usize = 20;
            ema = first_ema::<N>(&vec[0..N]);
        }

        let result = Candle::collection().update_one(doc! {"_id": last_id}, doc! {"$set": {ema_key.as_str(): ema}}, None)?;

        if result.modified_count == 0 {
            println!("error: update first ema{range} failure, symbol is {symbol}, interval is {interval},  id is [{last_id:?}] !");
        }
    }

    loop {
        let cursor = Candle::collection().find(doc!{"symbol": symbol, "interval" : interval, "time": {"$gte": time}}, FindOptions::builder().sort(doc!{"time": 1}).limit(1000).build())?;
    
        let mut index = 0;
        let mut prev_candle = None;

        for r in cursor {
            index += 1;
            if let Ok(x) = r {
                if index > 1 {
                    if let Some((id, ema10, ema20)) = prev_candle {

                        let mut update= doc! {};
                        let mut new_ema10 = None;
                        let mut new_ema20 = None;
                        if range == 10 {
                            if let Some(m) = ema10 {
                                new_ema10 = Some(next_ema(m, x.close, range));
                                update = doc!{"$set": {"ema10": new_ema10}};
                            }
                        }
                        else if range == 20 {
                            if let Some(m) = ema20 {
                                new_ema20 = Some(next_ema(m, x.close, range));
                                update = doc!{"$set": {"ema20": new_ema20}};
                            }
                        }

                        let ret = Candle::collection().update_one(doc!{"_id": x.id}, update, None)?;
                        
                        if ret.modified_count == 1 {
                            prev_candle = Some((x.id, new_ema10, new_ema20));
                            time = x.time;
                            println!("inf: update ema success! id={}", x.id);
                        }
                        else {
                            println!("error: update ema failure!, id={}", x.id);
                        }
                            
                    }
                    else {
                        println!("error: has no prev ema{}!, id={}", x.id, range);
                    }
                }
                else {
                    prev_candle = Some((x.id, x.ema10, x.ema20));
                }
            }
        }

        if index == 1 {
            println!("set ema end");
            break;
        }
    }

    Ok(())
}

fn proc_macd(symbol: &str, interval: &str) -> Result<()> {
    let mut time = 0_i64;
    let options = FindOneOptions::builder().sort(doc!{ "time": -1 }).build();
    let ret = Candle::collection().find_one(doc!{"symbol": symbol, "interval" : interval, "nmacd": {"$exists": true}}, options)?;

    if let Some(candle) = ret {
        time = candle.time;
    }
    else {
        let options = FindOptions::builder().sort(doc!{"time": 1}).limit(34).build();
        let cursor = Candle::collection().find(doc!{"symbol": symbol, "interval" : interval}, options)?;
    
        let mut index = 0;
        let mut vec:Vec<f64> = Vec::new();
        let mut last_id = None;
        for r in cursor {
            index += 1;

            let candle = r?;
            vec.push(candle.close);

            if index == 34 {
                last_id = Some(candle.id);
                time = candle.time;
            }
        }

        let (dif, dea, macd, ema12, ema26) = first_macd(&vec[0..34]);

        if let Some(id) = last_id {
            let nmacd = Macd { dif, dea, macd, ema12, ema26 };
            Candle::collection().update_one(doc!{"_id": id}, doc!{"$set": { "nmacd": bson::to_document(&nmacd)? }}, None)?;
        }
    }

    loop {
        let options = FindOptions::builder().sort(doc!{"time": 1}).limit(1000).build();
        let cursor = Candle::collection().find(doc!{"symbol": symbol, "interval" : interval, "time": {"$gte": time}}, options)?;
    
        let mut index = 0;
        let mut prev_candle = None;

        for r in cursor {
            index += 1;
            if let Ok(x) = r {
                if index > 1 {
                    if let Some((id, dif, dea, macd, ema12, ema26)) = prev_candle {
                        let (dif, dea, macd, ema12, ema26) = next_macd(dea, ema12, ema26, x.close);
                        let nmacd = Macd { dif, dea, macd, ema12, ema26 };
                        let ret = Candle::collection().update_one(doc!{"_id": x.id}, doc!{"$set": {"nmacd": bson::to_document(&nmacd)?}}, None)?;
                        if ret.modified_count == 1 {
                            prev_candle = Some((x.id, dif, dea, macd, ema12, ema26));
                            time = x.time;
                            println!("inf: update macd success! id={}", x.id);
                        }
                        else {
                            println!("error: update macd failure!, id={}", x.id);
                        }
                            
                    }
                    else {
                        println!("error: has no prev macd!, id={}", x.id);
                    }
                }
                else {
                    
                    if let Some(m) = x.nmacd {
                        prev_candle = Some((x.id, m.dif, m.dea, m.macd, m.ema12, m.ema26));
                    }
                    else {
                        println!("error: has no first macd!, id={}", x.id);
                    }
                }
            }
        }

        if index == 1 {
            println!("set macd end");
            break;
        }
    }
    Ok(())
}

fn percentile_linear_interpolation(arr: &[f64], percentage: i32) -> f64 {

    let index = (arr.len() - 1) as f64 * (percentage as f64 / 100 as f64);

    let floor = index.floor();
    let fraction = index - floor;

    // let candle_i = &candles[floor as usize];
    // let candle_j = &candles[floor as usize + 1];

    let i = arr[floor as usize];
    let j = arr[floor as usize + 1];

    i + ((j - i) * fraction)

}
 
fn median_candle_size(candles: &[Candle]) -> f64 {
    let mut vet: Vec<f64>= Vec::new();
    for candle in candles {
        let candle_size = (candle.high - candle.low).abs();
        // HashSet::new().sor
        vet.push(candle_size);
    }

    bubble_sort(&mut vet);

    if vet.len() % 2 == 0 {


        let f1 = vet[vet.len()/2];
        let f2 = vet[(vet.len() + 2)/2];

        (f1 + f2) / 2.0
    }
    else {
        let n = (vet.len() + 1) / 2;

        vet[n]
    }
}

fn bubble_sort(arr: &mut [f64]) {
    let mut tmp = 0.0;
    for i in (1..arr.len()).rev() {
        
        for j in 0..i {
            if arr[j] > arr[j + 1] {
                tmp = arr[j];
                arr[j] = arr[j + 1];
                arr[j + 1] = tmp
            }
        }
    }
}

fn highest(candles: &[Candle]) -> f64 {
    let mut high = candles[0].high;
    for candle in candles {
        if high < candle.high {
            high = candle.high;
        }
    }

    high
}

fn lowest(candles: &[Candle]) -> f64 {
    let mut low = candles[0].low;
    for candle in candles {
        if low > candle.low {
            low = candle.low;
        }
    }

    low
}

fn next_interval(interval: &str) -> &str{
    match interval {
        "5m" => "15m",
        "15m" => "30m",
        "30m" => "1h",
        "1h" => "2h",
        "2h" => "4h",
        "4h" => "1d",
        "8h" => "1d",
        "1d" => "1d",
        "3d" => "1w",
        _ => ""
    }
}

fn proc_cradle(candle: &Candle, prev_candles: &[Candle]) -> Result<i32> {
    let mut ema10 = 0.0;
    let mut ema20 = 0.0;

    if let Some(e10) = candle.ema10 {
        ema10 = e10;
    }
    if let Some(e20) = candle.ema20 {
        ema20 = e20;
    }

    let mut tag = 0;

    if ema10 > ema20 { 
        //2-进入cradle区域
        if check_overlap(ema20, ema10, candle.low, candle.high) {
            //3-是否是small candle
            if check_small_candle(candle, prev_candles) == 1 || check_small_candle(candle, prev_candles) == 0 {
                //4- bull candle
                if check_candle_bull_bear(candle) == 1 { //bull candle
                    tag = 1;
                }
            }
        }
    }
    else if ema20 > ema10 {
        //2-进入cradle区域
        if check_overlap(ema10, ema20, candle.low, candle.high) {
            //3-是否是small candle
            if check_small_candle(candle, prev_candles) == 1 || check_small_candle(candle, prev_candles) == 0 {
                //4- bear candle
                if check_candle_bull_bear(candle) == -1 { 
                    tag = -1
                }
            }
        }
    }

    if tag != 0 {
        let depth = DEPTH;
        let trend = check_trend_sample(candle.symbol.as_str(), candle.interval.as_str(), depth, candle.time)?;
        match trend {
            Trend::TrendUp => {
                if tag == 1 {
                    return Ok(1);
                }
            },
            Trend::TrendDown => {
                if tag == -1 {
                    return Ok(-1);
                }
            },
            _ => () //不存在趋势或背离
        }
        /* let trend = check_trend(candle.symbol.as_str(), candle.interval.as_str(), depth, candle.time)?;
        match trend {
            (Trend::TrendUp, MacdVergence::Convergence) => { //存在UP趋势; 趋势明显???
                 if tag == 1 { //candle看涨
                    //高时段存在相同趋势
                    let next_trend = check_trend(candle.symbol.as_str(), next_interval(candle.interval.as_str()), depth, candle.time)?;
                    if let (Trend::TrendUp,MacdVergence::Convergence)  = next_trend {
                        return Ok(1);
                    }
                 }
            },
            (Trend::TrendDown, MacdVergence::Convergence) => { //0-存在DOWN趋势; 趋势明显???
                if tag == -1 {//candle看跌
                    let next_trend = check_trend(candle.symbol.as_str(), next_interval(candle.interval.as_str()), depth, candle.time)?;
                    //高时段存在相同趋势
                    if let (Trend::TrendDown, MacdVergence::Convergence) = next_trend {
                        return Ok(-1);
                    }
                 }
            },
            (_,_) => () //不存在趋势或背离
        } */
    }

    Ok(0)

}

//检查是否区域重合, 进入cradle区域
fn check_overlap(low1: f64, high1: f64, low2: f64, high2:f64) -> bool {
    let low_max = low1.max(low2);
    let high_min = high1.min(high2);

    if high_min >= low_max {
        return true;
    }

    false
}

//检查蜡烛bull， bear
fn check_candle_bull_bear(candle: &Candle) -> i32 {
    //plotchar(close, title="Bull/Bear Candle", color=close>low+((high-low)*bbmulti)? green:close<high-((high-low)*bbmulti)? red:yellow, char='▴', location=location.belowbar, show_last=2)

    const BBMULTI:f64 = 0.6; 

    let tag = if candle.close > (candle.low + ((candle.high - candle.low) * BBMULTI)) {
        1
    }
    else if candle.close < (candle.high - ((candle.high - candle.low) * BBMULTI)) {
        -1
    }
    else {
        0
    };

    tag
}

//判断趋势， 是否背离， 趋势明显
fn check_trend(symbol: &str, interval: &str, depth: i32, time: i64) -> Result<(Trend, MacdVergence)> {

    let cursor = Zigzag::collection().find(doc! {"symbol": symbol, "interval": interval, "depth": depth, "time": {"$lte": time}}, FindOptions::builder().sort(doc! {"time": -1}).build())?;

    let mut high_gap_vet: Vec<f64> = Vec::new();
    let mut low_gap_vet: Vec<f64> = Vec::new();

    let mut next_high = 0.0;
    let mut next_low = 0.0;

    let mut high_macd_gap_vet: Vec<f64> = Vec::new();
    let mut low_macd_gap_vet: Vec<f64> = Vec::new();

    let mut next_macd_high = 0.0;
    let mut next_macd_low = 0.0;

    let mut tag = 0;

    let mut first_high = 0.0;
    let mut first_low = 0.0;


    for r in cursor {
        let zigzag = r?;

        if tag == 0 {
            tag = zigzag.tag;
        }

        if zigzag.tag == 1 { //高点
            if next_high != 0.0 {
                high_gap_vet.push(next_high - zigzag.value);
                if let Some(nmacd) = &zigzag.nmacd {
                    high_macd_gap_vet.push(next_macd_high - nmacd.dif);
                }
            }
            else {
                first_high = zigzag.value;
            }

            next_high = zigzag.value;
            if let Some(nmacd) = &zigzag.nmacd {
                next_macd_high = nmacd.dif;
            }
        }
        else { 
            if next_low != 0.0 {
                low_gap_vet.push(next_low - zigzag.value);
                if let Some(nmacd) = &zigzag.nmacd {
                    low_macd_gap_vet.push(next_macd_low - nmacd.dif);
                }
            }
            else {
                first_low = zigzag.value;
            }

            next_low = zigzag.value;
            if let Some(nmacd) = &zigzag.nmacd {
                next_macd_low = nmacd.dif;
            }
        }
    }

    const GAP_PERCENT: f64 = 0.05;
    let gap = (first_high - first_low).abs();

    let mut trend = Trend::TrendNo;
    let mut macd_vergence = MacdVergence::Divergence;

    if high_gap_vet.len() > 0 && low_gap_vet.len() > 0 && high_macd_gap_vet.len() > 0 && low_macd_gap_vet.len() > 0 {
        if high_gap_vet[0] * low_gap_vet[0] > 0.0 {
            if high_gap_vet[0] > 0.0 && high_macd_gap_vet[0] > 0.0 {
                trend = Trend::TrendUp;

                if high_gap_vet[0].abs() / gap > GAP_PERCENT  
                && low_gap_vet[0].abs() / gap > GAP_PERCENT{ //趋势明显，差距大于5%
                    macd_vergence = MacdVergence::Convergence;
                }
            }
            else if high_gap_vet[0] < 0.0 && low_macd_gap_vet[0] < 0.0 {
                trend = Trend::TrendDown;
                if high_gap_vet[0].abs() / gap > GAP_PERCENT  
                && low_gap_vet[0].abs() / gap > GAP_PERCENT{ //趋势明显，差距大于5%
                    macd_vergence = MacdVergence::Convergence;
                }
            }
        }
    }

    Ok((trend, macd_vergence))

}

//判断趋势， 是否背离， 趋势明显
fn check_trend_sample(symbol: &str, interval: &str, depth: i32, time: i64) -> Result<(Trend)> {

    let cursor = Zigzag::collection().find(doc! {"symbol": symbol, "interval": interval, "depth": depth, "time": {"$lte": time}}, FindOptions::builder().sort(doc! {"time": -1}).build())?;

    let mut high_gap_vet: Vec<f64> = Vec::new();
    let mut low_gap_vet: Vec<f64> = Vec::new();

    let mut next_high = 0.0;
    let mut next_low = 0.0;

    let mut tag = 0;

    let mut first_high = 0.0;
    let mut first_low = 0.0;


    for r in cursor {
        let zigzag = r?;

        if tag == 0 {
            tag = zigzag.tag;
        }

        if zigzag.tag == 1 { //高点
            if next_high != 0.0 {
                high_gap_vet.push(next_high - zigzag.value);
            }
            else {
                first_high = zigzag.value;
            }

            next_high = zigzag.value;
        }
        else { 
            if next_low != 0.0 {
                low_gap_vet.push(next_low - zigzag.value);
            }
            else {
                first_low = zigzag.value;
            }

            next_low = zigzag.value;
        }
    }

    const GAP_PERCENT: f64 = 0.05;
    let gap = (first_high - first_low).abs();

    let mut trend = Trend::TrendNo;

    if high_gap_vet.len() > 0 && low_gap_vet.len() > 0 {
        if high_gap_vet[0] * low_gap_vet[0] > 0.0 {
            if high_gap_vet[0] > 0.0 {
                trend = Trend::TrendUp;
            }
            else if high_gap_vet[0] < 0.0 {
                trend = Trend::TrendDown;
            }
        }
    }

    Ok((trend))

}
//判断是否是小candle
fn check_small_candle(candle: &Candle, prev_candles: &[Candle]) -> i32 {
    let mut ema10 = 0.0;
    let mut ema20 = 0.0;

    if let Some(e10) = candle.ema10 {
        ema10 = e10;
    }

    if let Some(e20) = candle.ema20 {
        ema20 = e20;
    }

    let trend_up = if ema10 > ema20 {
        true
    } else {
        false
    };
    
    let trend_dn = if ema10 < ema20 {
        true
    } else {
        false
    };

    let candle_size = (candle.high - candle.low).abs();

    let small_candles = &prev_candles[prev_candles.len()-4..prev_candles.len()];

    let highest4 = highest(small_candles); 
    let lowest4 = lowest(small_candles); 
    let stop_gap = if trend_dn && candle.high < highest4 {
        (highest4 - candle.low).abs()
    }
    else if trend_up && candle.low > lowest4 {
        (candle.high  - lowest4).abs()
    }
    else {
        0.0
    };
    
    //10根线取中位数
    let median_candles = &prev_candles[prev_candles.len()-10..prev_candles.len()];
    // let med = median_candle_size(median_candles);

    // let mut new_candles:Vec<&Candle> = Vec::new();
    // for i in (0..prev_candles.len()).rev() {
    //     new_candles.push(&median_candles[i]);
    // }

    let mut new_candles: Vec<f64>= Vec::new();
    for candle in median_candles {
        let candle_size = (candle.high - candle.low).abs();
        new_candles.push(candle_size);
    }

    bubble_sort(&mut new_candles);

    let med = percentile_linear_interpolation(&new_candles[0..10], 50);

    let tag = if candle_size > med {
        -1 //gray
    }
    else if stop_gap > med {
        0 //
    }
    else {
        1 //green
    };

    tag
}

fn proc_support(symbol: &str, interval: &str, depth: i32) -> Result<()> {

    let percent = 0.03;

    let cursor = Zigzag::collection().find(doc!{"symbol": symbol, "interval": interval, "depth": depth}, FindOptions::builder().sort(doc! {"time": 1}).build())?;

    let mut vec: Vec<Vec<f64>> = Vec::new();

    let mut highest = 0.0;
    let mut lowest: f64 = 0.0;

    for r in cursor {
        let zigzag = r?;
        let value = zigzag.value;

        if zigzag.tag == 1 {
            if highest == 0.0 {
                highest = value;
            }
            else if highest < value {
                highest = value;
            }
        }

        if zigzag.tag == -1 {
            if lowest == 0.0 {
                lowest = value;
            }
            else if lowest > value {
                lowest = value;
            }
        }
        
        let mut has = false;

        for v in vec.iter_mut() {
            let tmp = v[0];
            if ((value - tmp).abs() / value) < percent {
                let new_value = (value + tmp) / 2.0;
                v.push(value);
                v[0] = new_value;
                has = true;
                break;
            }
        }

        if !has {
            let v = vec![value, value];
            vec.push(v);
        }
    }

    let v = vec![highest, highest, highest];
    vec.push(v);

    let v = vec![lowest, lowest, lowest];
    vec.push(v);


    for v in vec {
        if v.len() > 2 {
            println!("---{v:?}")
        }
    }

    Ok(())
}

fn get_zigzag_stop_loss(symbol: &str, interval: &str, depth: i32, tag: i32, time: i64) -> Result<f64> {

    let mut stop_loss = 0.0;
    let mut d = doc!{"symbol": symbol, "interval": interval, "depth": depth, "tag": tag, "time": {"$lte": time}};
    let options = FindOneOptions::builder().sort(doc!{"time": -1}).build();

    let ret = Zigzag::collection().find_one(d, options)?;
    if let Some(zigzag) = ret {
        stop_loss = zigzag.value;
    }

    Ok(stop_loss)
} 

fn proc_hold_order(candle: &Candle, depth: i32) -> Result<()>{
    let cursor = Order::collection().find(doc! {"symbol": candle.symbol.as_str(), "interval": candle.interval.as_str()}, None)?;

    for r in cursor {
        let order = r?;

        let mut ziazag_tag = 0;
        if order.side == "BUY" {
            ziazag_tag = 1;
        }
        else {
            ziazag_tag = -1;
        }

        match order.status {
            0 => {
                if order.stop_loss > candle.low && order.stop_loss < candle.high { //触发stop_loss, 止损平仓
                    Order::collection().update_one(doc! {"_id": order.id}, doc! {"$set": {"status": -1}}, None)?;
                }
                else if order.target1 > candle.low && order.target1 < candle.high { //触发target1, update target2
                    // let target2 = 
                    let mut stop_loss = get_zigzag_stop_loss(candle.symbol.as_str(), candle.interval.as_str(), depth, ziazag_tag, candle.time)?;
                    //止盈一半， 更新stop_loss2

                    // if stop_loss != order.stop_loss2 {
                    //     Order::collection().update_one(doc! {"_id": order.id}, doc! {"$set": {"status": 1, "stop_loss2": stop_loss}}, None)?;
                    // }
                    if order.side == "BUY" {
                        if stop_loss < order.stop_loss {
                            stop_loss = order.stop_loss;
                        }
                    }
                    else {
                        if stop_loss > order.stop_loss {
                            stop_loss = order.stop_loss;
                        }
                    }

                    Order::collection().update_one(doc! {"_id": order.id}, doc! {"$set": {"status": 1, "stop_loss2": stop_loss}}, None)?;

                }
            }, 
            1 =>{
                
                if order.stop_loss2 > candle.low && order.stop_loss2 < candle.high { //触发stop_loss2， 止盈平仓
                    Order::collection().update_one(doc! {"_id": order.id}, doc! {"$set": {"status": 2}}, None)?;
                }
                else { //是否更新stop_loss2
                    let stop_loss = get_zigzag_stop_loss(candle.symbol.as_str(), candle.interval.as_str(), depth, ziazag_tag, candle.time)?;
                    
                    /* if stop_loss != order.stop_loss2 {
                        Order::collection().update_one(doc! {"_id": order.id}, doc! {"$set": {"stop_loss2": stop_loss}}, None)?;
                    } */
                    
                    if order.side == "BUY" {
                        if stop_loss > order.stop_loss2 {
                            Order::collection().update_one(doc! {"_id": order.id}, doc! {"$set": {"stop_loss2": stop_loss}}, None)?;
                        }
                    }
                    else {
                        if stop_loss < order.stop_loss2 {
                            Order::collection().update_one(doc! {"_id": order.id}, doc! {"$set": {"stop_loss2": stop_loss}}, None)?;
                        }
                    }
                }
            }
            _ => ()
        }
    }

    Ok(())
}

fn proc_make_order(candle: &Candle, entry: f64, side: &str, depth: i32) -> Result<bool>{

    /* let order = Order::collection().find_one(doc!{"symbol": candle.symbol.as_str(), "interval": candle.interval.as_str(), "status": 0}, None)?;

    if order.is_some() {
        return Ok(false);
    } */

    let mut stop_loss = 0_f64;
    let mut target1 = 0_f64;

    if side == "BUY" {
        stop_loss = get_zigzag_stop_loss(candle.symbol.as_str(), candle.interval.as_str(), depth, -1, candle.time)?;
        if stop_loss > 0_f64 {
            target1 = entry + (entry - stop_loss).abs();
        }
    }
    else if side == "SELL" {
        stop_loss = get_zigzag_stop_loss(candle.symbol.as_str(), candle.interval.as_str(), depth, 1, candle.time)?;
        if stop_loss > 0_f64 {
            target1 =  entry - (entry - stop_loss).abs();
        }
    }

    if stop_loss > 0_f64 && target1 > 0_f64 {
        let order = Order {
            id: ObjectId::new(),
            time: candle.time,
            symbol: candle.symbol.to_owned(),
            interval: candle.interval.to_owned(),
            time_date: candle.time_date.to_owned(),
            side: side.to_string(), // SELL, BUY
            entry,
            stop_loss,
            target1,
            stop_loss2: 0_f64,
            status: 0 //-1 亏损平仓； 0新单； 1 止盈一半平仓，不亏损； 2止盈平仓  
        };
        Order::collection().insert_one(order, None)?;

        return Ok(true);
    }
    
    Ok(false)
}

fn process(symbol: &str, interval: &str) -> Result<()>{

    proc_ema(symbol, interval, 10)?;
    proc_ema(symbol, interval, 20)?;
    proc_macd(symbol, interval)?;

    //

    let mut time = 0_i64;
    let start  = 10;

    loop {
        let cursor = Candle::collection().find(doc!{"symbol": symbol, "interval": interval, "time": {"$gte": time}}, FindOptions::builder().sort(doc!{"time": 1}).limit(500).build())?;
        let mut vec: Vec<Candle> = cursor.into_iter().map(|x| {
            let r = match x {
                Ok(a) => a,
                Err(e) => panic!()
            };
            r
        }).collect::<Vec<Candle>>();

        if vec.len() <= 10 {
            break;
        }

        for i in start..vec.len() {

            let candle = &vec[i];
            if i == vec.len() - start {
                time = candle.time;
            }

            proc_hold_order(candle, DEPTH)?;
            
            let prev_candles = &vec[(i-start+1)..i+1];

            // println!("{}", candle.time_date);
            // if candle.time_date == "2022-04-20T00:00:00.000Z" {
            //     println!("{}", candle.time_date);
            // }
            let check = proc_cradle(candle, prev_candles)?;
            if check != 0 {
                //make order
                println!("make order{}, {}", candle.time_date, check);
                let side = if check == 1 { "BUY" } else {"SELL"};

                proc_make_order(candle, candle.close, side, DEPTH)?;
            }
        }
    }

    Ok(())
    
}

fn proc_order_analysis(symbol: &str, interval: &str) -> Result<()> {
    let cursor = Order::collection().find(doc!{"symbol": symbol, "interval": interval, "side": "BUY"}, FindOptions::builder().sort(doc!{"time": 1}).build())?;

    let mut amout = 10000_f64;
    let percent = 0.01;

    for r in cursor {
        let order = r?;

        if order.status == -1 {
            amout -= (amout * percent);
        }
        else if order.status == 2 {
            let z = (amout * percent / 2_f64) / (order.stop_loss - order.entry);
            let m = (order.stop_loss2 - order.target1).abs() * z;

            amout += amout * percent / 2_f64;

            amout += amout * percent / 2_f64 + m;
        }
    }

    println!("amout = {}", amout);

    Ok(())
}

fn main() {
    let symbol = "BTCUSDT";
    let interval = "4h";

    // proc_support(symbol, interval, 12).unwrap();

    process(symbol, interval).unwrap();
    proc_order_analysis(symbol, interval).unwrap();
     
 
    // proc_ema(20).unwrap();

    // proc_macd().unwrap();

    // let mut arr = vec![2.0,5.0,4.0,6.0,3.0,7.0,9.0,8.0];

    // pao_sort(&mut arr[0..8]);

    // println!("{:?}", arr);
    // match proc_ema(20) {
    //     Ok(()) => print!("success"),
    //     Err(e) => println!("{}", e)
    // }
    // let vec = vec![4,5,6];
    // print!("{:?}", &vec[0..3]);
}