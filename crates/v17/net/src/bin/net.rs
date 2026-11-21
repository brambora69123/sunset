//! Deterministic connection-layer probe for later parity checks. Prints the
//! 1.7.10 wire forms, the crypto vectors and a full offline login over two
//! in-memory connections, one line per fact, no randomness outside the shared
//! `java.util.Random` port.

use sunset_1_7_core::LegacyRandom;
use sunset_1_7_net::buf::PacketBuf;
use sunset_1_7_net::connection::{Connection, PacketListener};
use sunset_1_7_net::crypto::{
    AesCfb8, RsaKeyPair, java_hex_bigint, public_key_der, random_bytes, server_id_hash,
};
use sunset_1_7_net::framing::frame;
use sunset_1_7_net::login::{GameProfile, HandshakeHandler, LoginContext, NoSessionService};
use sunset_1_7_net::packets::{Packet, Protocol};
use sunset_1_7_net::uuid::Uuid;
use sunset_1_7_net::{compress_frame, decompress_frame, split_frame};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn varint_line(value: i32) {
    let mut buf = PacketBuf::new();
    buf.write_var_int(value);
    println!(
        "varint {value} = {} [{} bytes]",
        hex(buf.written()),
        buf.len()
    );
}

fn main() {
    // 1. VarInt wire forms and sizes.
    varint_line(0);
    varint_line(127);
    varint_line(128);
    varint_line(2097151);
    varint_line(-1);
    // 2. Framing: a length-prefixed login start.
    let (_, payload) = Packet::LoginStart {
        name: "Notch".into(),
    }
    .encode(Protocol::Login, true)
    .expect("login start encodes");
    let framed = frame(&payload).expect("fits 21 bits");
    println!("login start frame = {}", hex(&framed));
    let mut stream = framed.clone();
    let split = split_frame(&mut stream).expect("one frame").expect("ready");
    println!("split back = {}", hex(&split));

    // 3. AES/CFB8: the keystream runs across updates (openssl-verified form).
    let key = [
        0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f,
        0x3c,
    ];
    let mut cipher = AesCfb8::new(&key);
    let mut login_frame = framed.clone();
    cipher.encrypt(&mut login_frame);
    println!("cfb8 login frame = {}", hex(&login_frame));

    // 4. The login nonce, offline UUIDs and the server-id digest.
    let mut random = LegacyRandom::new(42);
    let mut nonce = [0u8; 4];
    random_bytes(&mut random, &mut nonce);
    println!("nonce (seed 42) = {}", hex(&nonce));
    println!(
        "offline uuid Notch = {}",
        Uuid::offline_player("Notch").to_java_string()
    );
    let shared_secret: [u8; 16] = (0u8..16).collect::<Vec<u8>>().try_into().unwrap();
    let hash = server_id_hash("", &shared_secret, &shared_secret);
    println!("server id hash = {}", hex(&hash));
    println!("server id hash hex = {}", java_hex_bigint(&hash));
    let negative = [0x80u8, 0x00];
    println!("bigint hex [80 00] = {}", java_hex_bigint(&negative));

    // 5. RSA: the X.509 form and a full PKCS#1 round trip.
    let mut keygen_random = LegacyRandom::new(7);
    let keys = RsaKeyPair::generate_with_bits(256, &mut keygen_random).expect("keygen");
    let der = public_key_der(&keys.public_key());
    println!("rsa128 x509 der = {}", hex(&der));
    let message = b"hello";
    let ciphertext = keys
        .encrypt_pkcs1_v15(message, &mut keygen_random)
        .expect("encrypt");
    println!("rsa128 pkcs1 ct = {}", hex(&ciphertext));
    let round = keys.decrypt_pkcs1_v15(&ciphertext).expect("decrypt");
    println!("rsa128 decrypt = {}", String::from_utf8_lossy(&round));

    // 6. Compression: below and above the threshold.
    let small = compress_frame(b"hello", 256);
    println!("compress small (raw) = {}", hex(&small));
    let big: Vec<u8> = b"abcabcabcabc".iter().copied().cycle().take(512).collect();
    let big_frame = compress_frame(&big, 256);
    println!(
        "compress big = {} bytes, starts {}",
        big_frame.len(),
        hex(&big_frame[..4])
    );
    println!(
        "decompress big = {} bytes",
        decompress_frame(&big_frame).expect("inflate").len()
    );

    // 7. The full offline login over two in-memory connections.
    println!("--- offline login ---");
    let mut server_random = LegacyRandom::new(42);
    let server_keys = RsaKeyPair::generate_with_bits(256, &mut server_random).expect("keygen");
    let mut server = Connection::new(false);
    server.set_listener(HandshakeHandler::new(LoginContext::new(
        false,
        false,
        std::sync::Arc::new(server_keys),
        std::sync::Arc::new(NoSessionService),
        LegacyRandom::new(1234),
    )));
    let mut client = Connection::new(true);
    client.set_listener(Box::new(LoginRecorder::default()));
    client
        .send(Packet::Handshake {
            version: sunset_1_7_net::PROTOCOL_VERSION_1_7_10,
            address: "127.0.0.1".into(),
            port: 25565,
            next_protocol: Protocol::Login.id(),
        })
        .expect("handshake sends");
    println!("handshake frame = {}", hex(&client.take_outbound()));
    // The take above dropped the wire bytes; send again for the server.
    client
        .send(Packet::Handshake {
            version: sunset_1_7_net::PROTOCOL_VERSION_1_7_10,
            address: "127.0.0.1".into(),
            port: 25565,
            next_protocol: Protocol::Login.id(),
        })
        .expect("handshake sends");
    pump(&mut client, &mut server).expect("handshake pumps");
    println!("server protocol after handshake = {:?}", server.protocol());
    // The client login switches to the login state on its handshake intent.
    client.set_protocol(Protocol::Login);
    client
        .send(Packet::LoginStart {
            name: "Notch".into(),
        })
        .expect("login start sends");
    let outbound = client.take_outbound();
    println!("login start frame = {}", hex(&outbound));
    server.receive(&outbound).expect("server reads start");
    server.tick();
    // The second tick runs acceptLogin and sends the login success.
    server.tick();
    let success = server.take_outbound();
    println!("login success frame = {}", hex(&success));
    client.receive(&success).expect("client reads success");
    client.tick();
    let listener: Box<dyn std::any::Any> = client.take_listener().unwrap();
    let recorder = listener.downcast::<LoginRecorder>().unwrap();
    for packet in &recorder.packets {
        println!("client read = {packet:?}");
    }
    println!("profile = {:?}", GameProfile::offline("Notch"));
}

/// Records what the client side reads, for the probe output.
#[derive(Default)]
struct LoginRecorder {
    packets: Vec<Packet>,
}

impl PacketListener for LoginRecorder {
    fn handle(&mut self, packet: &Packet, _connection: &mut Connection) -> Result<(), String> {
        self.packets.push(packet.clone());
        Ok(())
    }
}

fn pump(client: &mut Connection, server: &mut Connection) -> Result<(), String> {
    let outbound = client.take_outbound();
    server
        .receive(&outbound)
        .map_err(|error| error.to_string())?;
    server.tick();
    let outbound = server.take_outbound();
    client
        .receive(&outbound)
        .map_err(|error| error.to_string())?;
    client.tick();
    Ok(())
}
