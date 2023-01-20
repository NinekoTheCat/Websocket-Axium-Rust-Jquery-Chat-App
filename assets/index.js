var socket = new WebSocket(`ws://${location.hostname}:3000/chat`);

window.addEventListener("load", function () {
  $(document).ready(readyFunc);
});
var chatWindow;
var chatField;
var usernameField;
var chatButton;
function readyFunc() {
  chatWindow = $(".chatMessages");
  chatWindow.empty();

  socket.onmessage = onResponse;
  // socket2.onmessage = onResponseSub;
  socket.onopen = async function () {
    socket.send(await cbor.encodeAsync("GetMessages"));
  };
  usernameField = $("#usrField");
  chatField = $("#msgCont");
  chatButton = $("#msgSendBut");
  chatButton.on("click", onClick);
  chatField.keypress(async function (e) {
    if (e.which == 13) {
      await onClick();
    }
  });
}

async function onClick() {
  let username = usernameField.val();
  let text = chatField.val();
  chatField.val("");
  if (typeof username !== 'string' || username.trim().length === 0) return;
  if (typeof text !== 'string' || text.trim().length === 0) return;
  socket.send(
    await cbor.encodeAsync({
      SendMessage: {
        "content": text,
        "author": username,
      },
    }),
  );
}
var messages = [];
async function onResponse(ev) {
  let data = ev.data;
  if (!(data instanceof Blob) || data.size <= 1) {
    return;
  }
  let decodableData = new Uint8Array(await data.arrayBuffer());
  let dataFinal = await cbor.decodeFirstSync(decodableData);
  if (dataFinal instanceof Array) {
    messages = dataFinal;
    while (messages.length > 10) {
      messages.shift();
    }
    displayChatMessages();
  } else {
    await handleSubscribedChatMessage(dataFinal);
  }
}
function reAddMissingHeaders() {
  let row = $("<tr/>");
  row.append($("<th/>", {
    text: "message",
  }));
  row.append($("<th/>", {
    text: "author",
  }));
  chatWindow.append(row);
}
function displayChatMessages() {
  chatWindow.empty();
  reAddMissingHeaders();
  for (const message of messages) {
    chatWindow.append(createMessageDomElementFromMessage(message));
  }
}

function createMessageDomElementFromMessage(message) {
  let row = $("<tr/>");
  row.append(`"<td>${message.content}</td>"`);
  row.append(`"<td>${message.author}</td>"`);
  return row;
}

function updateChatMessages(newMessage) {
  chatWindow.append(createMessageDomElementFromMessage(newMessage));
}

async function handleSubscribedChatMessage(dataFinal) {
  messages.push(dataFinal);
  if (messages.length === 1) {
    reAddMissingHeaders();
  }
  if (messages.length >= 10) {
    while (messages.length > 10) {
      messages.shift();
    }
    displayChatMessages();
  } else {
    updateChatMessages(dataFinal);
  }
}
