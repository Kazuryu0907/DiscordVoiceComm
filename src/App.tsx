import { useEffect, useState } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type VcType = { id: string; name: string };

function LabelSelect({
  setChannelId,
  vcs,
}: {
  setChannelId: React.Dispatch<React.SetStateAction<string>>;
  vcs: VcType[];
}) {
  return (
    <form className="max-w-sm mx-auto">
      <select
        id="labels"
        className="mx-auto bg-gray-50 text-center  border border-gray-300 text-gray-900 text-lg rounded-lg focus:ring-blue-500 focus:border-blue-500 block p-2.5"
        onChange={(e) => setChannelId(e.target.value as string)}
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
type IdentifyType = "Track1" | "Track2";
type EmitDataType = {
  user_id: string;
  event: "Join" | "Leave";
  identify: IdentifyType
  name: string;
};

type PubUserStateType = Map<string,{
  user_id: string,
  volume: number
}>;
const Users = ({ identify }: { identify: "Track1" | "Track2" }) => {
  const [pubUsers, setPubUsers] = useState<PubUserStateType>(new Map());
  const emitFn = (emit_data: EmitDataType) => {
    const { user_id, name } = emit_data;
    if (emit_data.event === "Join" && emit_data.identify === identify) {
      setPubUsers((users) => {
        users.set(name, {user_id,volume:100});
        return new Map(users);
      });
    } else if (emit_data.event === "Leave" && emit_data.identify === identify) {
      setPubUsers((users) => {
        users.delete(name);
        return new Map(users);
      });
    }
  };
  useEffect(() => {
    let unlisten: UnlistenFn;
    const fn = async () => {
      unlisten = await listen<EmitDataType>("user-data-changed", (event) => {
        emitFn(event.payload);
      });
    };
    fn();
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  console.log(Array.from(pubUsers.values()).map(u => u.volume));


  const UserIds = Array.from(pubUsers.keys()).map((name) => {

    const onChangeFn = (e:React.ChangeEvent<HTMLInputElement>) => {
      const volume = e.target.valueAsNumber;
      setPubUsers(users => {
          let old = pubUsers.get(name);
          // なんかエラー出たらそのまま返す
          if(!old)return pubUsers;
          users.set(name,{...old,volume: volume});
          return new Map(users);
      });
      invoke("update_volume",{user_id:pubUsers.get(name)?.user_id,volume:volume/100.});
    };
    return(
    <div key={name}>
      <div className="mx-5 relative">
        <p className="text-left">{name}</p>
        <input type="range" onChange={onChangeFn} className="w-full accent-amber-600" min={0} max={200} defaultValue={100} />
        <span className="text-sm text-gray-500 absolute start-0 -bottom-6">0</span>
        <span className="text-sm text-gray-500 absolute start-1/2 -translate-x-1/2 rtl:translate-x-1/2 -bottom-6">100</span>
        <span className="text-sm text-gray-500 absolute end-0 -bottom-6">200</span>
      </div>
    </div>
  )});
  return <div>{UserIds}</div>;
};


const Listening = ({identify}:{identify: IdentifyType}) => {
  const [listening,setListening] = useState(false);
  useEffect(()=>{
    type updateListeningType = {
      identify: IdentifyType,
      is_listening: boolean
    };
    const payload: updateListeningType = {
      identify,
      is_listening: listening
    };
    invoke("update_is_listening",payload);
  },[listening]);
  return(
    <div>
      <p>Listen</p>
      <input type="checkbox" checked={listening} onChange={e => setListening(e.target.checked)} />
    </div>
  )
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
      <h1 className="text-3xl font-black my-2">Welcome to DiscordVoiceComm</h1>
      <div className="mt-5 mx-auto font-bold text-lg">
        <p>Listener</p>
        <LabelSelect
          // channelId={subChannelId}
          setChannelId={setSubChannelId}
          vcs={vcs}
        />
      </div>
      <div className="grid grid-cols-2 mt-8">
        <div className=" font-bold text-lg">
          <Listening identify="Track1"/>
          <p>Speaker1</p>
          <LabelSelect
            // channelId={channelId}
            setChannelId={setChannelId}
            vcs={vcs}
          />
          <Users identify="Track1" />
        </div>
        <div className="flex-auto font-bold text-lg">
          <Listening identify="Track2"/>
          <p>Speaker2</p>
          <LabelSelect
            // channelId={channelId2}
            setChannelId={setChannelId2}
            vcs={vcs}
          />
          <Users identify="Track2" />
        </div>
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
