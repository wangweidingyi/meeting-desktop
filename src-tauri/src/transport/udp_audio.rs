use std::net::{ToSocketAddrs, UdpSocket};
use std::sync::{Arc, Mutex};

use crate::audio::AudioChunk;
use crate::protocol::udp_packet::{AudioSourceType, UdpAudioPacket};
use crate::transport::audio_transport::{AudioTransport, AudioUploadProgress};

pub trait UdpPacketSink: Send + Sync + Clone + 'static {
    fn send(&self, packet: &[u8]) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct UdpAudioTransport<S>
where
    S: UdpPacketSink,
{
    session_id: String,
    sink: S,
}

impl<S> UdpAudioTransport<S>
where
    S: UdpPacketSink,
{
    pub fn new(session_id: impl Into<String>, sink: S) -> Self {
        Self {
            session_id: session_id.into(),
            sink,
        }
    }

    pub fn sink(&self) -> &S {
        &self.sink
    }
}

impl<S> AudioTransport for UdpAudioTransport<S>
where
    S: UdpPacketSink,
{
    fn send_audio_chunk(&self, chunk: &AudioChunk) -> Result<AudioUploadProgress, String> {
        let packet =
            UdpAudioPacket::from_chunk(self.session_id.clone(), AudioSourceType::Mixed, chunk);
        let encoded = packet.encode()?;
        self.sink.send(&encoded)?;

        Ok(AudioUploadProgress {
            sequence: chunk.sequence,
            last_uploaded_mixed_ms: chunk.started_at_ms + u64::from(chunk.duration_ms),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryUdpSocket {
    packets: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl InMemoryUdpSocket {
    pub fn take_last_packet(&self) -> Option<Vec<u8>> {
        self.packets
            .lock()
            .ok()
            .and_then(|packets| packets.last().cloned())
    }

    pub fn packets(&self) -> Vec<Vec<u8>> {
        self.packets
            .lock()
            .map(|packets| packets.clone())
            .unwrap_or_default()
    }
}

impl UdpPacketSink for InMemoryUdpSocket {
    fn send(&self, packet: &[u8]) -> Result<(), String> {
        let mut packets = self.packets.lock().map_err(|error| error.to_string())?;
        packets.push(packet.to_vec());
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct NetworkUdpSocket {
    socket: Arc<UdpSocket>,
}

impl NetworkUdpSocket {
    pub fn connect(target: impl ToSocketAddrs) -> Result<Self, String> {
        let socket = UdpSocket::bind("0.0.0.0:0").map_err(|error| error.to_string())?;
        socket.connect(target).map_err(|error| error.to_string())?;

        Ok(Self {
            socket: Arc::new(socket),
        })
    }

    pub fn peer_addr(&self) -> Result<std::net::SocketAddr, String> {
        self.socket.peer_addr().map_err(|error| error.to_string())
    }
}

impl UdpPacketSink for NetworkUdpSocket {
    fn send(&self, packet: &[u8]) -> Result<(), String> {
        self.socket
            .send(packet)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::net::UdpSocket;
    use std::time::Duration;

    use crate::audio::AudioChunk;
    use crate::protocol::udp_packet::{AudioSourceType, UdpAudioPacket};
    use crate::transport::audio_transport::{AudioTransport, AudioUploadProgress};
    use crate::transport::test_support::lock_network_test;
    use crate::transport::udp_audio::{InMemoryUdpSocket, NetworkUdpSocket, UdpAudioTransport};

    #[test]
    fn udp_audio_transport_sends_mixed_packets_and_reports_progress() {
        let socket = InMemoryUdpSocket::default();
        let transport = UdpAudioTransport::new("meeting-1", socket.clone());
        let chunk = AudioChunk {
            sequence: 7,
            started_at_ms: 1_000,
            duration_ms: 200,
            payload: vec![9, 8, 7, 6],
        };

        let progress = transport.send_audio_chunk(&chunk).unwrap();
        let encoded = socket.take_last_packet().unwrap();
        let decoded = UdpAudioPacket::decode(&encoded).unwrap();

        assert_eq!(
            progress,
            AudioUploadProgress {
                sequence: 7,
                last_uploaded_mixed_ms: 1_200,
            }
        );
        assert_eq!(decoded.source_type, AudioSourceType::Mixed);
        assert_eq!(decoded.sequence, 7);
    }

    #[test]
    #[ignore = "requires local UDP socket permissions"]
    fn network_udp_socket_sends_encoded_packet_to_live_listener() {
        let _network_guard = lock_network_test();
        let listener = UdpSocket::bind("127.0.0.1:0").unwrap();
        listener
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let target = listener.local_addr().unwrap();

        let socket = NetworkUdpSocket::connect(target).unwrap();
        let transport = UdpAudioTransport::new("meeting-1", socket);
        let chunk = AudioChunk {
            sequence: 8,
            started_at_ms: 2_000,
            duration_ms: 200,
            payload: vec![1, 3, 5, 7],
        };

        let progress = transport.send_audio_chunk(&chunk).unwrap();

        let mut buffer = [0_u8; 2048];
        let (received, _) = listener.recv_from(&mut buffer).unwrap();
        let packet = UdpAudioPacket::decode(&buffer[..received]).unwrap();

        assert_eq!(
            progress,
            AudioUploadProgress {
                sequence: 8,
                last_uploaded_mixed_ms: 2_200,
            }
        );
        assert_eq!(packet.session_id, "meeting-1");
        assert_eq!(packet.sequence, 8);
        assert_eq!(packet.payload, vec![1, 3, 5, 7]);
    }
}
