// #[path = "../constant.rs"]
// mod constant;
// mod candle;

use crate::constant:: { MONGO_URL, MONGO_DB_NAME };

// use core::any::type_name;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use mongodb::{sync::{Client, Database, Collection}, options::ClientOptions, bson::{doc, Document, Bson, oid::ObjectId} };
use chrono::{Utc, DateTime} ;
use mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime;

fn get_db() -> Database {
    let client = Client::with_uri_str(MONGO_URL).unwrap();
    client.database(MONGO_DB_NAME)
}

static DB: Lazy<Database> = Lazy::new(|| get_db());

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub name: Option<String>,
    pub email: String,
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub time: DateTime<Utc>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub symbol: String,
    pub interval: String,
    pub time: i64,
    #[serde(rename = "open_value")]
    pub open: f64,
    #[serde(rename = "high_value")]
    pub high: f64,
    #[serde(rename = "low_value")]
    pub low: f64,
    #[serde(rename = "close_value")]
    pub close: f64,
    pub time_date: String,
    pub nmacd: Option<Macd>,
    pub ema10: Option<f64>,
    pub ema20: Option<f64>,
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub createTime: DateTime<Utc>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Macd {
    pub macd: f64,
    pub dea: f64,
    pub dif: f64,
    pub ema12: f64,
    pub ema26: f64
}

/* #[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle2 {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub symbol: String,
    pub interval: String,
    pub time: i64,
    #[serde(rename = "open_value")]
    pub open: f64,
    #[serde(rename = "high_value")]
    pub high: f64,
    #[serde(rename = "low_value")]
    pub low: f64,
    #[serde(rename = "close_value")]
    pub close: f64,
    pub time_date: String,
    pub nmacd: Option<Macd>,
    pub ema10: Option<f64>,
    pub ema20: Option<f64>,
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub createTime: DateTime<Utc>
} */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub time: i64,
    pub symbol: String,
    pub interval: String,
    pub time_date: String,
    pub side: String, // SELL, BUY
    pub entry: f64,
    pub stop_loss: f64,
    pub target1: f64,
    pub stop_loss2: f64,
    pub status: i32 //-1 亏损平仓； 0新单； 1 止盈一半平仓，不亏损； 2止盈平仓
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zigzag {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub time: i64,
    pub symbol: String,
    pub interval: String,
    pub depth: i32,
    pub value: f64,
    pub time_date: String,
    pub tag: i32, //H, L  1, -1
    pub nmacd: Option<Macd>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LtfSwing {
    pub time: i64,
    pub value: f64,
    pub time_date: String,
    pub breakout_time: i64,
    pub breakout_body_value: f64,
    pub breakout_time_date: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwingUnit {
    pub time: i64,
    pub time_date: String,
    pub value: f64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwingFvgUnit {
    pub time: i64,
    pub time_date: String,
    pub low: f64,
    pub high: f64
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SwingStatus {
    Watching = 0,
    WaitingBackFvg = 1,
    Closed = 2
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Trend {
    TrendDown = -1,
    TrendNo = 0,
    TrendUp = 1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MacdVergence {
    Convergence, //汇聚
    Divergence //背离
}

/* 
    //TryFrom: 实现数字转enum
    let status = SwingStatus::Watching as i32
    match status.try_info() {
        Ok(SwingStatus::Watching) => println!(""),
        _ => ()
    }
*/
impl TryFrom<i32> for SwingStatus {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            x if x == Self::Watching as i32 => Ok(Self::Watching), 
            x if x == Self::WaitingBackFvg as i32 => Ok(Self::WaitingBackFvg),
            x if x == Self::Closed as i32 => Ok(Self::Closed),
            _ => Err(())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwingFvg {
    pub start_time: i64,
    pub end_time: i64,
    pub start_time_date: String,
    pub end_time_date: String,
    pub start: f64,
    pub end: f64
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchSwing {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub time: i64,
    pub symbol: String,
    pub interval: String,
    pub value: f64,
    // pub value_str: String, 
    pub time_date: String,
    pub status: i32, //0 观察中 1FVG形成 2等待返回FVG 3关闭(下单/取消)
    pub tag: i32, //H, L  1, -1
    pub one: SwingUnit,
    pub two: SwingUnit,
    pub three: SwingUnit,

    pub ltf_swing: Option<LtfSwing>,
    pub fvg1: Option<SwingFvg>, 
    pub fvg2: Option<SwingFvg>,
    pub fvg3: Option<SwingFvg>,

    pub fb_zero_value: Option<f64> //fb 计算0线的价格, 即4小时突破后的最高或最低点， 出现最低点要更新此值
}

pub trait Operator<'a, T> 
where T: Serialize + DeserializeOwned + Unpin + Send + Sync
{
    fn model_name() -> &'a str;
    fn collection() -> Collection<T> {
        
        let collection = DB.collection::<T>(Self::model_name());
        collection
    }

    // fn insert(&self, o: T) -> Bson {
    //     self.collection().insert_one(o, None).unwrap().inserted_id
    // }

    // fn update_one(&self, filter: Document, doc: Document) -> Result<u64> {
    //     // self.collection().update_one(filter, doc, None).unwrap().modified_count

    //     let ret = self.collection().update_one(filter, doc, None);
    //     match ret {
    //         Ok(r) => Ok(r.modified_count),
    //         Err(e) => Err(ModelError(e.to_string()))
    //     }
    // }

    // fn find_one(&self, filter: Document) -> T {
    //     self.collection().find_one(filter, None).unwrap().unwrap()
    // }

}


impl <'a> Operator<'a, User> for User {
    fn model_name() -> &'a str { "users" }
}

impl <'a> Operator<'a, Candle> for Candle {
    fn model_name() -> &'a str { "candlesticks9" }
}

/* impl <'a> Operator<'a, Candle2> for Candle2 {
    fn model_name() -> &'a str { "candlesticks9" }
}
 */
impl <'a> Operator<'a, Zigzag> for Zigzag {
    fn model_name() -> &'a str { "zigzag" }
}

impl <'a> Operator<'a, WatchSwing> for WatchSwing {
    fn model_name() -> &'a str { "watch_swing" }
}

impl <'a> Operator<'a, Order> for Order {
    fn model_name() -> &'a str { "order" }
}

