use smpp::result::Result;

mod test_utils;

use test_utils::TestClient;
use test_utils::TestServer;

#[test]
fn listens_on_tcp_port() -> Result<()> {
    // Given a server
    let server = TestServer::start()?;
    server.runtime.block_on(async {
        // When we connect
        let mut client = TestClient::connect_to(&server).await?;
        // Then we can write and read to it
        client.write_str("foo").await?;
        let resp = client.read_string().await?;
        assert!(resp.len() > 0);
        Ok(())
    })
}

#[test]
fn disconnects_clients_when_overloaded() -> Result<()> {
    // Given a server that allows <=2 clients
    let server = TestServer::start()?;
    server.runtime.block_on(async {
        // When we connect 3 clients
        let mut client1 = TestClient::connect_to(&server).await?;
        let mut client2 = TestClient::connect_to(&server).await?;
        let mut client3 = TestClient::connect_to(&server).await?;
        client1.write_str("foo").await?;
        client2.write_str("foo").await?;
        client3.write_str("foo").await?;
        let resp1 = client1.read_string().await?;
        let resp2 = client2.read_string().await?;
        let resp3 = client3.read_string().await?;
        // Then two of they are able to stay connected
        assert!(resp1.len() > 0);
        assert!(resp2.len() > 0);
        // And the third gets immediately disconnected
        assert!(resp3.len() == 0);
        Ok(())
    })
}
