#[macro_use]
extern crate may;
extern crate may_signal;

fn main() {
    let s = may_signal::ctrl_c();
    for _ in s.iter().take(3) {
        println!("CTRL_C pressed!");
    }

    join!(
        {
            let s = may_signal::ctrl_c();
            for _ in s.iter().take(3) {
                println!("CTRL_C pressed! in coroutine 0");
            }
        },
        {
            let s = may_signal::ctrl_c();
            for _ in s.iter().take(4) {
                println!("CTRL_C pressed! in coroutine 1");
            }
        }
    );

    #[cfg(unix)]
    {
        let sig_int = may_signal::Signal::new(may_signal::unix::SIGINT).unwrap();
        let sig_trm = may_signal::Signal::new(may_signal::unix::SIGTERM).unwrap();
        for _ in 0..3 {
            select!(
                _ = sig_int.recv().unwrap() => println!("SIGINT received"),
                _ = sig_trm.recv().unwrap() => println!("SIGTRM received")
            );
        }
    }
}
