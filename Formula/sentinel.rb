# Homebrew Formula for Sentinel
# To install: brew install engramai-io/tap/sentinel
# Or from local: brew install --build-from-source ./Formula/sentinel.rb

class Sentinel < Formula
  desc "Secure audit logging and observability for MCP (Model Context Protocol) servers"
  homepage "https://github.com/EngramAI-io/Core"
  url "https://github.com/EngramAI-io/Core/archive/refs/tags/v0.2.0.tar.gz"
  sha256 "UPDATE_THIS_SHA256_AFTER_CREATING_RELEASE"
  license "MIT"
  head "https://github.com/EngramAI-io/Core.git", branch: "main"

  depends_on "rust" => :build
  depends_on "node" => :build

  def install
    # Build frontend
    cd "frontend/my-react-flow-app" do
      system "npm", "install"
      system "npm", "run", "build"
    end

    # Build Rust binary
    system "cargo", "build", "--release"
    
    # Install binary
    bin.install "target/release/sentinel"
    
    # Install shell completions
    generate_completions_from_executable(bin/"sentinel", "completions")
  end

  def caveats
    <<~EOS
      Sentinel has been installed successfully!
      
      Quick start:
      
      1. Generate signing keys:
         sentinel keygen --out-dir ~/.sentinel/keys
      
      2. Run with an MCP server:
         sentinel run --signing-key-b64-path ~/.sentinel/keys/signing_key.b64 \\
           -- npx @modelcontextprotocol/server-filesystem
      
      3. For production, use authentication:
         sentinel run --ws-token "$(openssl rand -hex 32)" \\
           --signing-key-b64-path ~/.sentinel/keys/signing_key.b64 \\
           -- your-mcp-server
      
      Documentation: https://github.com/EngramAI-io/Core/blob/main/README.md
      
      Security features:
      - WebSocket authentication (--ws-token)
      - Cryptographic audit logging (Ed25519 signatures)
      - PII redaction
      - Graceful shutdown with flush guarantees
    EOS
  end

  test do
    # Test that the binary is installed and runs
    assert_match "sentinel", shell_output("#{bin}/sentinel --version")
    
    # Test keygen command
    system bin/"sentinel", "keygen", "--out-dir", testpath/"keys"
    assert_predicate testpath/"keys/signing_key.b64", :exist?
    assert_predicate testpath/"keys/signing_pubkey.b64", :exist?
    
    # Test recipient keygen
    system bin/"sentinel", "recipient-keygen", "--out-dir", testpath/"keys"
    assert_predicate testpath/"keys/recipient_priv.b64", :exist?
    assert_predicate testpath/"keys/recipient_pub.b64", :exist?
  end
end
