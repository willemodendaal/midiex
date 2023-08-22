defmodule WillemTest do
  use ExUnit.Case, async: true

  @tag will: true
  test "send a note to a port" do
    # This works on Windows 10.

    Midiex.ports()
    |> IO.inspect(label: "Ports")

    port = Midiex.ports(:output) |> Enum.find(fn p -> p.name == "loopMIDI Port" end)
    out_conn = Midiex.open(port)

    for i <- 1..1000 do
    Midiex.send_msg(out_conn, Midiex.Message.note_on(:D4))

      IO.write(".")
      :timer.sleep(1000)
    end
  end

end
