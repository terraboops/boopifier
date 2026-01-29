class BoopifierBin < Formula
  desc "Universal notification handler for Claude Code events (pre-built binary)"
  homepage "https://github.com/terraboops/boopifier"
  license "Apache-2.0"

  on_arm do
    url "https://github.com/terraboops/boopifier/releases/download/v0.2.2/boopifier-aarch64-apple-darwin.tar.gz"
    sha256 "355d3a4f6178404e43ea8ffb1d26f32d1ec0e9ff33ddd1b821f7b6698fee3ec1"
  end

  on_intel do
    url "https://github.com/terraboops/boopifier/releases/download/v0.2.2/boopifier-x86_64-apple-darwin.tar.gz"
    sha256 "2d81b3dbdfa4f159ed99def767a03b6fbfdfc21cbd768357acfe0beb9a52d6f4"
  end

  conflicts_with "boopifier", because: "both install a `boopifier` binary"

  def install
    bin.install "boopifier"
  end

  test do
    # Test that boopifier runs and shows help
    assert_match "Universal notification handler", shell_output("#{bin}/boopifier --help")
  end

  def caveats
    <<~EOS
      Boopifier requires configuration in ~/.claude/boopifier.json
      See documentation at: https://github.com/terraboops/boopifier

      Optional dependencies for full functionality:
      - notify-rust: Desktop notifications (works on macOS Notification Center)
      - rodio: Sound playback (CoreAudio on macOS)
      - signal-cli: Signal messenger integration (brew install signal-cli)

      Example usage:
        echo '{"hook_event_name": "Notification", "message": "test"}' | boopifier
    EOS
  end
end
