import { Component, useState } from 'react'
import { Box, Button, List, ListItem, ListItemText, Menu, MenuItem, styled, TextField, Tooltip, Typography } from '@mui/material';
import Container from '@mui/material/Container';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import SendIcon from '@mui/icons-material/Send';
import ArrowDropDownIcon from '@mui/icons-material/ArrowDropDown';

type MessageType = "system" | "user" | "error";
type AppState = {
    inputV             : string,
    socket             : null | WebSocket,
    commandListEl      : null | HTMLElement,
    commandListOpen    : boolean,
    name               : string,
    room               : string,
    history            : JSX.Element[]
};

type WSMessage = {
    message        : string,
    sender_type    : string,
    sender_name    : string,
    set_info       : string[]
};

const MBox = styled(Box)(({ theme }) => ({
    display: "flex",
    gap: "1em",
    [theme.breakpoints.down("sm")]: {
        flexDirection: "column",
    },
}));

type CommandListMenuProps = {
    onMenuItemClick: (what: string) => void
}
function CommandListMenu(props: CommandListMenuProps) {
    const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null);
    const [isOpen, setIsOpen] = useState<boolean>(false);
    let commands = [
        ["/name <name>", "Set name", "/name "],
        ["/join <room name>", "Join a room", "/join "],
        ["/leave", "Leave a room", "/leave"],
        ["/list rooms", "List available rooms", "/list rooms"],
        ["/list users", "List all users", "/list users"]
    ];
    let menuitems: JSX.Element[] = [];

    for (let i=0;i<commands.length;i++) {
        menuitems.push(
            <Tooltip key={`commands-list-item-${i}`} title={commands[i][1]} placement='right-start'>
                <MenuItem onClick={() => {
                    props.onMenuItemClick(commands[i][2]);
                    setIsOpen(false);
                }}>{commands[i][0]}</MenuItem>
            </Tooltip>
        )
    }
    return <Tooltip title="List available commands" placement='top-start'>
        <div>
        <Button
            id='commands-list'
            variant='contained'
            aria-controls={isOpen ? "commands-list-menu" : undefined}
            aria-haspopup="true"
            aria-expanded={isOpen ? "true" : undefined}
            onClick={(ev) => {
                setAnchorEl(ev.currentTarget);
                setIsOpen(true);
            }}
            endIcon={<ArrowDropDownIcon />}
        >
            Commands
        </Button>
        <Menu
            id="commands-list-menu"
            anchorEl={anchorEl}
            open={isOpen}
            onClose={() => setIsOpen(false)}
        >
            {menuitems.map((item) => ( item ))}
        </Menu>
        </div>
    </Tooltip>
}

export class App extends Component {
    state: AppState = {
        inputV: "",
        socket: null,
        commandListEl: null,
        commandListOpen: false,
        name: "",
        room: "",
        history: [],
    }

    onSocketclose = (ev: CloseEvent) => {
        console.log("SocketClose: ", ev);
        let hist = this.state.history;
        if (ev.reason.length !== 0) {
            hist.push(this.list(ev.reason, "system", "system"));
        }
        this.setState(prev => ({
            ...prev,
            socket: null,
            name: "",
            room: "",
            history: hist
        }));
    };

    onSocketErr = (ev: Event) => {
        console.log("SocketErr: ", ev);
        if (this.state.socket)
            this.state.socket.close(0);
    }

    onSocketMsg = (e: MessageEvent) => {
        if ((e.data as string).length == 0) {
            return;
        }
        let data = JSON.parse(e.data) as WSMessage;
        console.log("data", data);
        let hist = this.state.history;

        let update = {};
        if (data.set_info != null) {
            update["name"] = data.set_info[0];
            update["room"] = data.set_info[1];
        }
        if (data.message.length != 0) {
            hist.push(this.list(data.message, data.sender_name, data.sender_type as MessageType));
            update["history"] = hist;
        }
        console.log("update", update);
        this.setState(prev => ({
            ...prev,
            ...update
        }));
    }

    toggleConnect() {
        if (this.state.socket) {
            this.state.socket.close();
            return;
        }
        let uri = import.meta.env.VITE_WEBSOCKET_URI as string;
        if (uri.length == 0) {
            let hist = this.state.history;
            hist.push(this.list(
                "[Error no WEBSOCKET_URI env key]",
                "error", "system"
            ));
            this.setState(prev => ({
                ...prev,
                history: hist
            }));
            return;
        }
        let socket = new WebSocket(uri);
        console.log(socket);

        socket.onopen = () => {
            socket.send("/get-info");
        };

        socket.onclose = this.onSocketclose;

        socket.onerror = this.onSocketErr;
        // socket.onopen = (e) => {
        // };
        socket.onmessage = this.onSocketMsg;
        this.setState(prev => ({
            ...prev,
            socket
        }));
    }

    getUserInfo() {
        if (this.state.socket == null || this.state.socket.readyState !== WebSocket.OPEN) {
            return;
        }
        this.state.socket.send("/get-info");
    }

    send() {
        if (this.state.inputV.trim().length != 0) {
            if (this.state.socket) {
                this.state.socket.send(this.state.inputV.trim());
            }
            this.state.inputV = "";
            this.setState(prev => ({
                ...prev,
                inputV: ""
            }));
        }
    }

    setInputV(v: string) {
        this.setState(prev => ({
            ...prev,
            inputV: v
        }));
    }

    list(message: string, sender_name: string, sender_type: MessageType) {
        let bg = "grey.800";
        let color = "white";
        if (sender_type == "system") {
            bg = "warning.main";
            color = "black";
        }
        else if (sender_type == "error") {
            bg = "error.dark";
        }
        let msg = `${sender_name} : ${message}`;
        return <ListItem role="listitem" sx={{ padding: 0 }} key={`${self.crypto.randomUUID()}`}>
            <ListItemText
                primary={msg}
                sx={{ backgroundColor: bg, whiteSpace: "pre-wrap", color: color, padding: "0.6em", margin: 0 }} />
        </ListItem>
    }

    render() {

        let connectionColor: 'success' | 'error' = (this.state.socket)? 'success':'error';
        let connectionMsg = (this.state.socket)? 'Connected':'Disconnected';
        let connectionTooltip = (this.state.socket)? "Connected to websocket":'Disconnected to websocket';

        return <>
            <Typography variant='h1' gutterBottom sx={{ textAlign: "center" }}>Chat</Typography>
            <Container>
                <Card variant='elevation' sx={{ height: 'inherit' }}>
                    <CardContent>
                        <MBox sx={{ marginBottom: "0.7em", alignItems: "center" }}>
                            <Typography
                                sx={(theme) => ({
                                    [theme.breakpoints.down("sm")]: {
                                        width: 1
                                    },
                                })}
                            >
                                Name: {this.state.name}</Typography>
                            <Typography
                                sx={(theme) => ({
                                    [theme.breakpoints.down("sm")]: {
                                        width: 1
                                    },
                                })}
                            >
                                Room: {this.state.room}</Typography>
                            <Box display="flex" gap="1em"
                                sx={(theme) => ({
                                    flexGrow: 1,
                                    [theme.breakpoints.down("sm")]: {
                                        width: 1,
                                        "&>*, &>*>button": {
                                            width: 1
                                        }
                                    },
                                    ["@media screen and (max-width: 370px)"]: {
                                        flexDirection: "column",
                                    }
                                })}>
                                <CommandListMenu
                                    onMenuItemClick={( (what: string) => {
                                        this.setState(prev => ( {
                                            ...prev,
                                            inputV: what
                                        } ));
                                    } ).bind(this)}
                                />
                                <Tooltip title={connectionTooltip} placement='top-start' sx={{marginLeft: "auto"}}>
                                    <Button variant="contained" color={connectionColor} onClick={this.toggleConnect.bind(this)}>{connectionMsg}</Button>
                                </Tooltip>
                            </Box>
                        </MBox>
                        <Box sx={(theme) => ({
                            padding: 0,
                            border: '1px solid grey',
                            height: "50svh",
                            overflow: "auto",
                            [theme.breakpoints.down("sm")]: {
                                height: "35svh",
                            }
                        })}>
                            <List role="listbox" dense={true} sx={{ padding: 0 }}>
                                {this.state.history.map((item) => ( item ))}
                            </List>
                        </Box>
                        <MBox sx={{ marginTop: "0.7em"}}>
                            <TextField
                                value={this.state.inputV} onKeyUp={(e) => {
                                    if (e.key == "Enter") {
                                        e.preventDefault();
                                        this.send();
                                    }
                                }}
                                onChange={(e) => this.setInputV(e.target.value)}
                                label="Send a message or a command (command starts with '/')"
                                variant="outlined"
                                sx={(theme) => ({
                                    width: 0.9,
                                    [theme.breakpoints.down("sm")]: {
                                        width: 1
                                    }
                                })}
                            />
                            <Tooltip title="Send a message or a command">
                                <Button
                                    onClick={this.send.bind(this)}
                                    variant='text' sx={{ px: "2em" }}
                                    startIcon={<SendIcon />}>Send</Button>
                            </Tooltip>
                        </MBox>
                    </CardContent>
                </Card>
            </Container>
        </>;
    }
}
