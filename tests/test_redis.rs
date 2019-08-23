use actix::prelude::*;
use actix_redis::{Error, RedisActor, RedisCmd, RedisValue};
use futures::Future;

#[test]
fn test_error_connect() -> std::io::Result<()> {
    let sys = System::new("test");

    let addr = RedisActor::start("redis://localhost:54000");
    let _addr2 = addr.clone();

    Arbiter::spawn_fn(move || {
        addr.send(RedisCmd::Get("test".to_string())).then(|res| {
            match res {
                Ok(Err(Error::NotConnected)) => (),
                _ => panic!("Should not happen {:?}", res),
            }
            System::current().stop();
            Ok(())
        })
    });

    sys.run()
}

#[test]
fn test_redis() -> std::io::Result<()> {
    let _ = env_logger::builder().is_test(true).try_init();
    let sys = System::new("test");

    let addr = RedisActor::start("redis://127.0.0.1:6379");
    let _addr2 = addr.clone();

    Arbiter::spawn_fn(move || {
        let addr2 = addr.clone();
        addr.send(RedisCmd::Set("test".to_string(), "value".to_string()))
            .then(move |res| match res {
                Ok(Ok(resp)) => {
                    assert_eq!(resp, RedisValue::Okay);
                    addr2.send(RedisCmd::Get("test".to_string())).then(|res| {
                        match res {
                            Ok(Ok(resp)) => {
                                println!("RESP: {:?}", resp);
                                assert_eq!(
                                    resp,
                                    RedisValue::Data((&b"value"[..]).into())
                                );
                            }
                            _ => panic!("Should not happen {:?}", res),
                        }
                        System::current().stop();
                        Ok(())
                    })
                }
                _ => panic!("Should not happen {:?}", res),
            })
    });

    sys.run()
}

#[test]
fn test_redis_with_expire() -> std::io::Result<()> {
    let _ = env_logger::builder().is_test(true).try_init();
    let sys = System::new("test");

    let addr = RedisActor::start("redis://127.0.0.1:6379");
    let _addr2 = addr.clone();

    Arbiter::spawn_fn(move || {
        let addr2 = addr.clone();
        addr.send(RedisCmd::SetWithEx("test".to_string(), "value".to_string(), 60))
            .then(move |res| match res {
                Ok(Ok(resp)) => {
                    assert_eq!(resp, RedisValue::Okay);
                    addr2.send(RedisCmd::Get("test".to_string())).then(|res| {
                        match res {
                            Ok(Ok(resp)) => {
                                println!("RESP: {:?}", resp);
                                assert_eq!(
                                    resp,
                                    RedisValue::Data((&b"value"[..]).into())
                                );
                            }
                            _ => panic!("Should not happen {:?}", res),
                        }
                        System::current().stop();
                        Ok(())
                    })
                }
                _ => panic!("Should not happen {:?}", res),
            })
    });

    sys.run()
}
