import { useCallback, useEffect, useState } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
// import "./App.css";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { PlayIcon, PauseIcon } from "lucide-react";
import { debounce } from "lodash";

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
      <Select onValueChange={(value) => setChannelId(value)}>
        <SelectTrigger className="mx-auto text-center text-lg">
          <SelectValue placeholder="Select a channel" />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            {vcs.map((vc) => {
              return (
                <SelectItem key={vc.id} value={vc.id}>
                  {vc.name}
                </SelectItem>
              );
            })}
          </SelectGroup>
        </SelectContent>
      </Select>
    </form>
  );
}
type IdentifyType = "Track1" | "Track2";
type EmitDataType = {
  user_id: string;
  event: "Join" | "Leave";
  identify: IdentifyType;
  name: string;
};

type PubUserStateType = Map<
  string,
  {
    user_id: string;
    volume: number;
  }
>;
const Users = ({ identify,updater }: { identify: "Track1" | "Track2",updater:boolean }) => {
  // UserのVC Sliderをリセットするために，強制Re-render用のupdater
  const [pubUsers, setPubUsers] = useState<PubUserStateType>(new Map());
  const emitFn = (emit_data: EmitDataType) => {
    const { user_id, name } = emit_data;
    if (emit_data.event === "Join" && emit_data.identify === identify) {
      setPubUsers((users) => {
        users.set(name, { user_id, volume: 100 });
        return new Map(users);
      });
    } else if (emit_data.event === "Leave" && emit_data.identify === identify) {
      setPubUsers((users) => {
        users.delete(name);
        return new Map(users);
      });
    }
  };
  // updaterが変化したら，Usersをリセットする
  useEffect(() => {
    setPubUsers(() => new Map());
    console.log("reseted");
  },[updater]);

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
  const debounceTime = 300;
  const onChangeVolume = useCallback(
    debounce((value: number[], name: string) => {
      const volume = value[0];
      setPubUsers((users) => {
        let old = pubUsers.get(name);
        // なんかエラー出たらそのまま返す
        if (!old) return pubUsers;
        users.set(name, { ...old, volume: volume });
        return new Map(users);
      });
      invoke("update_volume", {
        user_id: pubUsers.get(name)?.user_id,
        volume: volume / 100,
      });
    },debounceTime),
    [pubUsers]
  );
  const UserIds = Array.from(pubUsers.keys()).map((name) => {
    return (
      <div key={name} className="mt-5">
        <div className="mx-5 relative">
          <p className="text-left">{name}</p>
          <Slider
            onValueChange={(value) => onChangeVolume(value, name)}
            defaultValue={[100]}
            max={200}
            step={1}
          />
          <span className="text-sm text-gray-500 absolute start-0 -bottom-6">
            0
          </span>
          <span className="text-sm text-gray-500 absolute start-1/2 -translate-x-1/2 rtl:translate-x-1/2 -bottom-6">
            100
          </span>
          <span className="text-sm text-gray-500 absolute end-0 -bottom-6">
            200
          </span>
        </div>
      </div>
    );
  });
  return <div>{UserIds}</div>;
};

const Listening = ({ identify }: { identify: IdentifyType }) => {
  const [listening, setListening] = useState(false);
  useEffect(() => {
    type updateListeningType = {
      identify: IdentifyType;
      is_listening: boolean;
    };
    const payload: updateListeningType = {
      identify,
      is_listening: listening,
    };
    invoke("update_is_listening", payload);
  }, [listening]);
  return (
    <div>
      <p>{listening ? "Now Listening" : "Stopping"}</p>
      <Button
        onClick={() => setListening((l) => !l)}
        variant="outline"
        size="icon"
      >
        {listening ? <PauseIcon /> : <PlayIcon />}
      </Button>
    </div>
  );
};

function App() {
  const [vcs, setVCs] = useState<VcType[]>([]);
  const [channelId, setChannelId] = useState<string>("");
  const [channelId2, setChannelId2] = useState<string>("");
  const [subChannelId, setSubChannelId] = useState<string>("");
  const [usersUpdater,setUsersUpdater] = useState<boolean>(false);

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
  const cleanUpUsers = () => {
    setUsersUpdater(u => !u);
  };
  return (
    <main className="text-center">
      <h1 className="text-3xl font-black my-2">Welcome to DiscordVoiceComm</h1>
      <div className="mt-5 mx-auto font-bold text-lg">
        <p>Listener</p>
        <LabelSelect setChannelId={setSubChannelId} vcs={vcs} />
      </div>
      <div className="grid grid-cols-2 mt-8">
        <div className=" font-bold text-lg">
          <Listening identify="Track1" />
          <p>Speaker1</p>
          <LabelSelect setChannelId={setChannelId} vcs={vcs} />
          <Users identify="Track1" updater={usersUpdater} />
        </div>
        <div className="flex-auto font-bold text-lg">
          <Listening identify="Track2" />
          <p>Speaker2</p>
          <LabelSelect setChannelId={setChannelId2} vcs={vcs} />
          <Users identify="Track2" updater={usersUpdater} />
        </div>
      </div>
      <div className="mt-10">
        <Button className="mx-5" onClick={onJoin}>
          <p className="text-lg px-5 font-bold">Join</p>
        </Button>
        <Button
          className="mx-5"
          onClick={() => invoke("leave").then(cleanUpUsers).catch(console.error)}
        >
          <p className="text-lg px-4 font-bold">Leave</p>
        </Button>
      </div>
    </main>
  );
}

export default App;
