from collections import deque, defaultdict


USERS = {}
ROOMS = {}


class Room(object):

    def __init__(self, name):
        self.name = name
        self.counter = 0
        self.messages = deque(maxlen=64)

    def add(self, author, text):
        self.counter += 1
        data = {'id': self.counter, 'text': text}
        self.messages.append(data)
        return data

    def get_history(self, first_id):
        return [m for m in self.messages if m['id'] > first_id]


class User(object):

    def __init__(self, uid, **props):
        self.uid = uid
        self.__dict__.update(props)
        self.rooms_last_seen = defaultdict(int)

    def update(self, meta):
        self.__dict__.update(meta)

    def initial_lattice(self):
        shared = {}
        mine = {}
        for k, n in self.rooms_last_seen.items():
            shared[k] = {
                'last_message_counter': ROOMS[k].counter}
            mine[k] = {'last_seen_counter': n}
        return {
            'shared': shared,
            'private': {
                self.uid: mine,
            }
        }

    def add_room(self, room):
        if room in self.rooms_last_seen:
            return {}
        if not room in ROOMS:
            r = ROOMS[room] = Room(room)
        else:
            r = ROOMS[room]
        self.rooms_last_seen[room] = r.counter
        return {
            'shared': { room: {'last_message_counter': r.counter} },
            'private': {
                self.uid: {
                    room: {'last_seen_counter': self.rooms_last_seen[room]},
                },
            }
        }


def ensure_user(uid, **meta):
    if uid not in USERS:
        user = USERS[uid] = User(uid, **meta)
    else:
        user = USERS[uid]
        user.update(meta)
    return user


def get_user(uid):
    return USERS.get(uid)

def get_room(room):
    return ROOMS.get(room)
