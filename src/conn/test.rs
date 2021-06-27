#[cfg(test)]
mod test {

    use crate::{config::Config, conn::*};
    use tokio::time::timeout;
    use util::Error;

    #[tokio::test]
    async fn test_multiple_close() -> Result<(), Error> {
        let server_a = DnsConn::server(
            SocketAddr::new(Ipv4Addr::new(0, 0, 0, 0).into(), 5353),
            Config::default(),
        )?;

        server_a.close().await?;

        if let Err(err) = server_a.close().await {
            assert_eq!(err, *ERR_CONNECTION_CLOSED);
        } else {
            assert!(false, "expected error, but got ok");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_query_respect_timeout() -> Result<(), Error> {
        let server_a = DnsConn::server(
            SocketAddr::new(Ipv4Addr::new(0, 0, 0, 0).into(), 5353),
            Config::default(),
        )?;

        let (a, b) = mpsc::channel(1);

        timeout(Duration::from_millis(100), a.send(()))
            .await
            .unwrap()
            .unwrap();

        let res = server_a.query("invalid-host", b).await;
        assert_eq!(
            res.clone().err(),
            Some(ERR_CONNECTION_CLOSED.to_owned()),
            "server_a.query expects timeout!"
        );

        server_a.close().await?;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_query_interval() -> Result<(), Error> {
        let query_socket =
            DnsConn::create_socket(SocketAddr::new(Ipv4Addr::new(0, 0, 0, 0).into(), 5353))
                .unwrap();

        let query_addr = query_socket.local_addr().unwrap();

        println!("{:?}", query_addr);

        let query_server = DnsConn::initialize_server(query_socket, Config::default()).unwrap();

        let (query_close_channel_send, query_close_channel_recv) = mpsc::channel(1);

        tokio::spawn(async move {
            query_server
                .query("invalid-host.local", query_close_channel_recv)
                .await
                .unwrap();

            query_server.close().await.unwrap();
        });

        let socket =
            DnsConn::create_socket(SocketAddr::new(Ipv4Addr::new(0, 0, 0, 0).into(), 5353))
                .unwrap();

        let mut data = vec![0u8; INBOUND_BUFFER_SIZE];

        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(4)) => {
                    return Err(Error::new("failed to recv connection.".to_owned()))
                },

                result = socket.recv_from(&mut data) => {
                    match result{
                        Ok((_, addr)) => {
                            println!("received");
                            if addr == query_addr {
                                query_close_channel_send.send(()).await.unwrap();
                                return Ok(());
                            }
                        },

                        Err(err) => {
                            return Err(Error::new(err.to_string()));
                        },
                    }
                }
            }
        }
    }
}
