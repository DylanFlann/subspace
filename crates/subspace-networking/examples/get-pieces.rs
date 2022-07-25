use futures::channel::oneshot;
use libp2p::multiaddr::Protocol;
use libp2p::multihash::Code;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;
use subspace_core_primitives::{crypto, FlatPieces, Piece, PieceIndexHash, U256};
use subspace_networking::{
    Config, PiecesByRangeRequest, PiecesByRangeRequestHandler, PiecesByRangeResponse, PiecesToPlot,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config_1 = Config {
        listen_on: vec!["/ip4/0.0.0.0/tcp/0".parse().unwrap()],
        value_getter: Arc::new(|key| {
            // Return the reversed digest as a value
            Some(key.digest().iter().copied().rev().collect())
        }),
        request_response_protocols: vec![PiecesByRangeRequestHandler::create(|req| {
            println!("Request handler for request: {:?}", req);

            let piece_bytes: Vec<u8> = Piece::default().into();
            let flat_pieces = FlatPieces::try_from(piece_bytes).unwrap();
            let pieces = PiecesToPlot {
                piece_indexes: vec![1],
                pieces: flat_pieces,
            };

            let response = Some(PiecesByRangeResponse {
                pieces,
                next_piece_index_hash: Some(PieceIndexHash::from([0; 32])),
            });

            println!("Sending response... ");

            std::thread::sleep(Duration::from_secs(1));
            response
        })],
        allow_non_globals_in_dht: true,
        ..Config::with_generated_keypair()
    };
    let (node_1, mut node_runner_1) = subspace_networking::create(config_1).await.unwrap();

    println!("Node 1 ID is {}", node_1.id());

    let (node_1_address_sender, node_1_address_receiver) = oneshot::channel();
    let on_new_listener_handler = node_1.on_new_listener(Arc::new({
        let node_1_address_sender = Mutex::new(Some(node_1_address_sender));

        move |address| {
            if matches!(address.iter().next(), Some(Protocol::Ip4(_))) {
                if let Some(node_1_address_sender) = node_1_address_sender.lock().take() {
                    node_1_address_sender.send(address.clone()).unwrap();
                }
            }
        }
    }));

    tokio::spawn(async move {
        node_runner_1.run().await;
    });

    // Wait for first node to know its address
    let node_1_addr = node_1_address_receiver.await.unwrap();
    drop(on_new_listener_handler);

    let config_2 = Config {
        bootstrap_nodes: vec![node_1_addr.with(Protocol::P2p(node_1.id().into()))],
        listen_on: vec!["/ip4/0.0.0.0/tcp/0".parse().unwrap()],
        allow_non_globals_in_dht: true,
        ..Config::with_generated_keypair()
    };

    let (node_2, mut node_runner_2) = subspace_networking::create(config_2).await.unwrap();

    println!("Node 2 ID is {}", node_2.id());

    tokio::spawn(async move {
        node_runner_2.run().await;
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    let hashed_peer_id = PieceIndexHash::from(crypto::sha256_hash(&node_1.id().to_bytes()));

    // obtain closest peers to the middle of the range
    let key = libp2p::multihash::MultihashDigest::digest(
        &Code::Identity,
        &U256::from(hashed_peer_id).to_be_bytes(),
    );
    let peer_id = node_2.get_closest_peers(key).await.unwrap()[0];

    let pieces = node_2
        .send_generic_request(
            peer_id,
            PiecesByRangeRequest {
                start: hashed_peer_id,
                end: hashed_peer_id,
            },
        )
        .await;

    if let Ok(pieces) = pieces {
        println!("Piece found: {pieces:?}");
    }

    println!("Exiting..");
}
