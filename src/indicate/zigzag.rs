
// #[path = "../db/mod.rs"]
// mod db;
// #[path = "../common.rs"]
// mod common;

use std::collections::{ HashMap, HashSet };
// use common::{ CustomError, Result };
// use db::Candle;
// use db::User;

// use crate::Candle;
// use crate::User;
use crate::{ CustomError, Result, Candle };

pub struct ZigzagMapBuffer {
    pub low_map: HashMap<i64, f64>,
    pub high_map: HashMap<i64, f64>,
    pub zigzag_map: HashMap<i64, f64>
}

pub fn zigzag(candles: &Vec<Candle>, buffer: &mut ZigzagMapBuffer, step_size: f64, depth: usize) -> Result<(HashSet<usize>, HashMap<usize, char>)> {
    
    const INTERVAL: u32 = 15;
    const DEVIATION: f64 = 5.0;
    const BACKSTEP: usize = 2;

    const LEVEL: u8 = 3; //回退拐点数量

    let mut rm_set: HashSet<usize> = HashSet::new();

    let counted_bars = buffer.zigzag_map.len(); //已计算的bar数量

    let mut next = 'U'; // L, H 下个获取标识

    let mut limit_index = 0;

    let mut cur_low: f64 = 0.0; 
    let mut cur_high: f64 = 0.0;
    let mut last_low: f64 = 0.0;
    let mut last_high: f64 = 0.0;

    let total_count = candles.len();

    /*
        1.对计算位置进行初期化
        1.1判断是否是第一次进行高低点计算，如果是，则设定计算位置为除去ExtDepth个图形最初的部分。
        1.2如果之前已经计算过，找到最近已知的三个拐点（高点或低点），将计算位置设置为倒数第三个拐点之后，重新计算最后的拐点。
    */

    if counted_bars == 0 { //第一次
        limit_index = depth - 1;
    }
    else {
        let mut count = 0;
        let mut i = total_count - 1;

        while count < LEVEL && i > total_count - 1 - 100  {
            
            if let Some(x) = candles.get(i) {
                if let Some(x) = buffer.zigzag_map.get(&x.time) {
                    if *x != 0.0 {
                        count += 1;
                    }
                    
                }
            }
            i -= 1;
        }
        i += 1;
        limit_index = i;
        if let Some(x) = candles.get(limit_index) {
            if let Some(y) = buffer.low_map.get(&x.time) {
                cur_low = *y;
                next = 'H';
            }
            else if let Some(y) = buffer.high_map.get(&x.time) {
                cur_high = *y;
                next = 'L';
            }
        }

        for i in limit_index+1..total_count { //重新计算第三个拐点之后
            if let Some(x) = candles.get(i) {
                if let Some(_) = buffer.zigzag_map.remove(&x.time) {

                    rm_set.insert(i);

                    buffer.low_map.remove(&x.time);
                    buffer.high_map.remove(&x.time);
                    
                }
            }
        }
    }
    
    

    /* 
        2.从步骤1已经设置好的计算位置开始，将对用于存储高低点的变量进行初始化，准备计算高低点

        2.1计算ExtDepth区间内的低点，如果该低点是当前低点，则进行2.1.1的计算，并将其记录成一个低点。
        2.1.1如果当前低点比上一个低点值小于相对点差(ExtDeviation)；并且之前ExtBackstep个Bars的记录的中，高于当前低点的值清空。
        2.2高点的计算如同2.1以及分支处理2.1.1。
    */
    
    for i in limit_index..total_count {

        if let Some(x) = candles.get(i) {
            if x.time == 1630886400000 {
                println!(">>>")
            }
        }

        //low
        let mut low = 0.0;
        let mut low_time = 0;
        let (index, mut lowest, time) = lowestbars(&candles,  i + 1 - depth, depth)?;
        if lowest == last_low {
            lowest = 0.0
        }
        else {
            last_low = lowest;
            if let Some(x) = candles.get(i) {
                low = x.low;
                low_time = x.time;

                if low - lowest > DEVIATION * step_size {
                    lowest = 0.0
                }
                else {
                    for j in 1..=BACKSTEP {
                        if let Some(y) = candles.get(i + j) {
                            if let Some(_) = buffer.low_map.get(&y.time) {
                                buffer.low_map.remove(&y.time);
                            }
                        }
                    }
                }
            }
        }
        if low == lowest {
            buffer.low_map.insert(low_time, lowest);
        }

        //high
        let mut high = 0.0;
        let mut high_time = 0;
        
        let (index, mut highest, time) = highestbars(&candles, i + 1 - depth, depth)?;
        if highest == last_high {
            highest = 0.0
        }
        else {
            last_high = highest;
            if let Some(x) = candles.get(i) {
                high = x.high;
                high_time = x.time;

                if highest - high > DEVIATION * step_size {
                    highest = 0.0
                }
                else {
                    for j in 1..=BACKSTEP{
                        if let Some(y) = candles.get(i + j) {
                            if let Some(_) = buffer.high_map.get(&y.time) {
                                buffer.high_map.remove(&y.time);
                            }
                        }
                    }
                }
            }
        }
        if high == highest {
            buffer.high_map.insert(high_time, highest);
        }

        if next == 'U' {
            last_low = 0.0;
            last_high = 0.0;
        }
        else {
            last_low = cur_low;
            last_high = cur_high;
        }
    }

    /*
        3.从步骤1已经设置好的计算位置开始，定义指标高点和低点
        3.1如果开始位置为高点，则接下来寻找低点，在找到低点之后，将下一个寻找目标定义为高点
        3.2如果开始位置为低点，则与3.1反之。
    */

    let mut last_high_pos = 0;
    let mut last_low_pos = 0;
    let mut add_map: HashMap<usize, char> = HashMap::new(); // H 高点, L 低点

    for i in limit_index..total_count {

        if let Some(x) = candles.get(i) {
            if x.time == 1630886400000 {
                println!("");
            }
            match next {
                'U' => {
                    if last_low == 0.0 && last_high == 0.0 {
                        if let Some(h) = buffer.high_map.get(&x.time) {
                            next = 'L';
                            last_high_pos = i;
                            last_high = *h;
                            buffer.zigzag_map.insert(x.time, last_high);
                            add_map.insert(i, 'H');
                        }
                        if let Some(l) = buffer.low_map.get(&x.time) {
                            next = 'H';
                            last_low_pos = i;
                            last_low = *l;
                            buffer.zigzag_map.insert(x.time, last_low);
                            add_map.insert(i, 'L');
                        }
                    }
                },
                'H' => {
                    if let Some(y) = buffer.low_map.get(&x.time) {
                        if *y < last_low {
                            // if let None = buffer.high_map.get(&x.time) {

                                if let Some(z) = candles.get(last_low_pos) {
                                    if let Some(_) = buffer.zigzag_map.remove(&z.time) {
                                        add_map.remove(&last_low_pos);
                                    }
                                }
                                last_low_pos = i;
                                last_low = *y;
                                buffer.zigzag_map.insert(x.time, last_low);
                                add_map.insert(i, 'L');
                            // }
                        }
                    }
                    if let Some(y) = buffer.high_map.get(&x.time) {
                        if let None = buffer.low_map.get(&x.time) {
                            last_high = *y;
                            last_high_pos = i;
                            buffer.zigzag_map.insert(x.time, last_high);
                            add_map.insert(i, 'H');
                            next = 'L';
                        }
                    }
                },
                'L' => {
                    if let Some(y) = buffer.high_map.get(&x.time) {
                        if *y > last_high {
                            // if let None = buffer.low_map.get(&x.time) {
                                if let Some(z) = candles.get(last_high_pos) {
                                    if let Some(_) = buffer.zigzag_map.remove(&z.time) {
                                        add_map.remove(&last_high_pos);
                                    }
                                }
                                last_high_pos = i;
                                last_high = *y;
                                buffer.zigzag_map.insert(x.time, last_high);
                                add_map.insert(i, 'H');
                            // }
                        }
                    }
                    if let Some(y) = buffer.low_map.get(&x.time) {
                        if let None = buffer.high_map.get(&x.time) {
                            last_low = *y;
                            last_low_pos = i;
                            buffer.zigzag_map.insert(x.time, last_low);
                            add_map.insert(i, 'L');
                            next = 'H';
                        }
                    }
                },
                _ => todo!()
            }
        }
    }


    // let high = 0.0f64;
    // let low = 0.0f64;

    // let mut bars: Vec<Candle> = Vec::new();

    // // let subbars:&Vec<Candle>  = &bars[0..12];

    // // let highest = highestbars()

    // let start = 0;

    // let highest_index = highestbars(&bars, depth, depth)?;
    // let lowest_index = lowestbars(&bars, depth, depth)?;





    // if highest_index - lowest_index > 0 {

    // }

    for index in 0..candles.len() {
        let current = &candles[index];
        if index < 12  {
            let high = current.high;
            let low = current.low;

        }
    }

    // let candles: [&str; 1] = ["111"];

    Ok((rm_set, add_map))
}


fn highestbars(bars: &Vec<Candle>, start: usize, length: usize) -> Result<(usize, f64, i64)>{ 
    if start >= bars.len() {
        let m = format!("highestbars start is bigger than vec length: start is {}, vec length is {}", start, bars.len());
        return Err(Box::new(CustomError(m.to_string())));
    }
    if length > bars.len() - start {
        let m = format!("highestbars length is bigger than expect vec length: length is {}, expect vec length is {}", length, bars.len() - start);
        return Err(Box::new(CustomError(m.to_string())));
    }
    if bars.len() > 0 {
        let firstBar = &bars[start];
        let mut highest = firstBar.high;
        let mut index: usize = start;
        let mut time: i64 = firstBar.time;
        let end = start + length;
        for i in start..end {
            let bar = &bars[i];
            let high: f64 = bar.high;
            if high >= highest {
                index = i;
                highest = high;
                time = bar.time;
            }
        }
        return Ok((index, highest, time));
    }
    Err(Box::new(CustomError("vec is empty".to_string())))
}

fn lowestbars(bars: &Vec<Candle>, start: usize, length: usize) -> Result<(usize, f64, i64)>{
    if start >= bars.len() {
        let m = format!("lowestbars start is bigger than vec length: start is {}, vec length is {}", start, bars.len());
        return Err(Box::new(CustomError(m.to_string())));
    }
    if length > bars.len() - start {
        let m = format!("lowestbars length is bigger than expect vec length: length is {}, expect vec length is {}", length, bars.len() - start);
        return Err(Box::new(CustomError(m.to_string())));
    }
    if bars.len() > 0 {
        let firstBar = &bars[start];
        let mut lowest = firstBar.low;
        let mut index:usize = start;
        let mut time: i64 = firstBar.time;
        let end = start + length;
        for i in start..end {
            let bar = &bars[i];
            let low: f64 = bar.low;
            if low <= lowest {
                index = i;
                lowest = low;
                time = bar.time;
            }
        }
        return Ok((index, lowest, time));
    }
    Err(Box::new(CustomError("vec is empty".to_string())))
}
