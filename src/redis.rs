use actix::fut::{err, ok};
use actix::prelude::*;
use backoff::backoff::Backoff;
use backoff::ExponentialBackoff;
use redis;

pub use redis::Value as RedisValue;

use crate::Error;

pub enum RedisCmd {
    Set(String, String),
    SetWithEx(String, String, i64),
    Get(String),
    Del(String),
}

impl Message for RedisCmd {
    type Result = Result<redis::Value, Error>;
}

pub struct RedisActor {
    addr: String,
    backoff: ExponentialBackoff,
    client: Option<redis::aio::SharedConnection>,
}

impl RedisActor {
    pub fn start<S: Into<String>>(addr: S) -> Addr<RedisActor> {
        let addr = addr.into();

        let mut backoff = ExponentialBackoff::default();
        backoff.max_elapsed_time = None;

        Supervisor::start(|_| RedisActor {
            addr,
            backoff,
            client: None,
        })
    }
}

impl Actor for RedisActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        let client = redis::Client::open(self.addr.as_ref()).unwrap();
        client
            .get_shared_async_connection()
            .into_actor(self)
            .map(|con, act, _| {
                info!("Connected to redis server: {}", act.addr);
                act.client = Some(con);
                act.backoff.reset();
            })
            .map_err(|err, act, ctx| {
                error!("Can not connect to redis server: {}", err);
                if let Some(timeout) = act.backoff.next_backoff() {
                    ctx.run_later(timeout, |_, ctx| ctx.stop());
                }
            })
            .wait(ctx);
    }
}

impl Supervised for RedisActor {
    fn restarting(&mut self, _: &mut Self::Context) {
        self.client.take();
    }
}

impl Handler<RedisCmd> for RedisActor {
    type Result = ResponseActFuture<Self, redis::Value, Error>;

    fn handle(&mut self, msg: RedisCmd, _: &mut Self::Context) -> Self::Result {
        if let Some(con) = self.client.clone() {
            let res = match msg {
                RedisCmd::Get(key) => redis::cmd("GET").arg(key).query_async(con),
                RedisCmd::Del(key) => redis::cmd("DEL").arg(key).query_async(con),
                RedisCmd::Set(key, val) => {
                    redis::cmd("SET").arg(key).arg(val).query_async(con)
                }
                RedisCmd::SetWithEx(key, val, ttl) => redis::cmd("SET")
                    .arg(key)
                    .arg(val)
                    .arg("EX")
                    .arg(ttl)
                    .query_async(con),
            }
            .map(|t| t.1)
            .into_actor(self)
            .then(|res, act, ctx| match res {
                Ok(res_ok) => ok(res_ok),
                Err(res_err) => {
                    if res_err.is_io_error() {
                        warn!("Redis connection error: {} error: {}", act.addr, res_err);
                        ctx.stop();
                    }
                    err(Error::Redis(res_err))
                }
            });
            Box::new(res)
        } else {
            Box::new(err(Error::NotConnected))
        }
    }
}
