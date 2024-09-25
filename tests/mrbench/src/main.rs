use clap::Parser;
use futures::future::FutureExt;
use lol_tests::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Parser)]
struct Opts {
    #[clap(long, short = 'n', default_value_t = 1)]
    num_nodes: u8,
    #[clap(long, short = 'p', default_value_t = 1)]
    num_shards: u32,
    #[clap(long="du", short='t', value_parser = humantime::parse_duration, default_value = "1s")]
    io_duration: Duration,
    #[clap(long, short = 'w', default_value_t = 1)]
    n_par_writes: u32,
    #[clap(long, short = 'r', default_value_t = 1)]
    n_par_reads: u32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    dbg!(&opts);

    let cluster = Arc::new(Cluster::new(opts.num_nodes, opts.num_shards).await?);

    let t = Instant::now();
    let mut futs = vec![];
    for shard_id in 0..opts.num_shards {
        let cluster = cluster.clone();
        let fut = async move {
            cluster.add_server(shard_id, 0, 0).await?;
            for node_id in 1..opts.num_nodes {
                cluster.add_server(shard_id, 0, node_id).await?;
            }
            Ok::<(), anyhow::Error>(())
        };
        futs.push(fut);
    }
    futures::future::try_join_all(futs).await?;
    eprintln!("cluster setup: {:?}", t.elapsed());

    let t = Instant::now();
    let du = opts.io_duration;
    while t.elapsed() < du {
        let fail_w = Arc::new(AtomicU64::new(0));
        let fail_r = Arc::new(AtomicU64::new(0));
        let mut futs = vec![];
        for shard_id in 0..opts.num_shards {
            for _ in 0..opts.n_par_writes {
                let error_counter = fail_w.clone();
                let mut cli = cluster.user(0);
                let fut = async move {
                    if let Err(_) = cli.fetch_add(shard_id, 1).await {
                        error_counter.fetch_add(1, Ordering::Relaxed);
                    }
                }
                .boxed();
                futs.push(fut);
            }
        }
        for shard_id in 0..opts.num_shards {
            for _ in 0..opts.n_par_reads {
                let error_counter = fail_r.clone();
                let cli = cluster.user(0);
                let fut = async move {
                    if let Err(_) = cli.read(shard_id).await {
                        error_counter.fetch_add(1, Ordering::Relaxed);
                    }
                }
                .boxed();
                futs.push(fut);
            }
        }
        let t = Instant::now();
        futures::future::join_all(futs).await;

        let error_w = fail_w.load(Ordering::Relaxed);
        let error_r = fail_r.load(Ordering::Relaxed);
        eprintln!(
            "io done. {:?}. error=(w={error_w},r={error_r})",
            t.elapsed()
        );
    }

    eprintln!("done");
    Ok(())
}