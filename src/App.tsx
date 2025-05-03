import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type VcType = { id: string; name: string };

function LabelSelect({
  channelId,
  setChannelId,
  vcs,
}: {
  channelId: string;
  setChannelId: React.Dispatch<React.SetStateAction<string>>;
  vcs: VcType[];
}) {
  return (
    <form className="max-w-sm mx-auto">
      <select
        id="labels"
        className="mx-auto bg-gray-50 text-center  border border-gray-300 text-gray-900 text-lg rounded-lg focus:ring-blue-500 focus:border-blue-500 block p-2.5"
        onChange={(e) => setChannelId(e.target.value as string)}
        value={channelId}
      >
        {vcs.map((vc) => {
          return (
            <option key={vc.id} value={vc.id}>
              {vc.name}
            </option>
          );
        })}
      </select>
    </form>
  );
}

function App() {
  const [vcs, setVCs] = useState<VcType[]>([]);
  const [channelId, setChannelId] = useState<string>("");
  const [channelId2, setChannelId2] = useState<string>("");
  const [subChannelId, setSubChannelId] = useState<string>("");

  useEffect(() => {
    const fn = async () => {
      const voice_channels: VcType[] = await invoke("get_voice_channels");
      console.log(voice_channels);
      setVCs(voice_channels);
      setChannelId(voice_channels[0].id);
      setChannelId2(voice_channels[0].id);
      setSubChannelId(voice_channels[0].id);
    };
    fn();
  }, []);

  const onJoin = async () => {
    console.log(channelId, channelId2, subChannelId);
    await invoke("join", {
      ch1: channelId,
      ch2: channelId2,
      sub_ch: subChannelId,
    });
  };

  return (
    <main className="container">
      <h1 className="text-3xl font-black my-7">Welcome to DiscordVoiceComm</h1>
      <div className="flex mt-8">
        <div className="flex-auto font-bold text-lg">
          <p>Speaker1</p>
          <LabelSelect
            channelId={channelId}
            setChannelId={setChannelId}
            vcs={vcs}
          />
        </div>
        <div className="flex-auto font-bold text-lg">
          <p>Speaker2</p>
          <LabelSelect
            channelId={channelId2}
            setChannelId={setChannelId2}
            vcs={vcs}
          />
        </div>
      </div>
      <div className="mt-5 mx-auto font-bold text-lg">
        <p>Listener</p>
        <LabelSelect
          channelId={subChannelId}
          setChannelId={setSubChannelId}
          vcs={vcs}
        />
      </div>
      <button className="mt-15 mx-auto" onClick={onJoin}>
        <p className="text-lg px-5 font-bold">Join</p>
      </button>
      <button className="mt-5 mx-auto" onClick={() => invoke("leave")}>
        <p className="mx-auto px-5 font-bold">Leave</p>
      </button>
    </main>
  );
}

export default App;
