# Loads MSD_musicnn_big and MSD_musicnn once, then tags files.
# One file:  python musicnn-one.py song.m4a
# Batch:     python musicnn-one.py --batch
#            stdin: one path per line; stdout: one JSON object per line.

from __future__ import print_function

import json
import os
import sys
import traceback

os.environ.setdefault("TF_CPP_MIN_LOG_LEVEL", "3")

import numpy as np
import librosa
import tensorflow as tf

tf.compat.v1.disable_eager_execution()
tf.compat.v1.logging.set_verbosity(tf.compat.v1.logging.ERROR)

from musicnn import configuration as config
from musicnn import models
from musicnn.extractor import batch_data


def n_frames_for(length=3.0):
    return librosa.time_to_frames(
        length, sr=config.SR, n_fft=config.FFT_SIZE, hop_length=config.FFT_HOP
    ) + 1


def load_session(model_name, n_frames):
    graph = tf.Graph()
    with graph.as_default():
        x = tf.compat.v1.placeholder(tf.float32, [None, n_frames, config.N_MELS])
        is_training = tf.compat.v1.placeholder(tf.bool)
        y, _, _, _, _, _, _, _, _ = models.define_model(
            x, is_training, model_name, len(config.MSD_LABELS)
        )
        normalized_y = tf.nn.sigmoid(y)
        sess = tf.compat.v1.Session(graph=graph)
        sess.run(tf.compat.v1.global_variables_initializer())
        saver = tf.compat.v1.train.Saver()
        ckpt = os.path.join(os.path.dirname(models.__file__), model_name) + "/"
        saver.restore(sess, ckpt)
    return sess, x, is_training, normalized_y


def run_model(sess, x, is_training, y, batch):
    chunks = []
    size = config.BATCH_SIZE
    for start in range(0, batch.shape[0], size):
        tags = sess.run(
            y,
            feed_dict={x: batch[start : start + size], is_training: False},
        )
        chunks.append(tags)
    taggram = np.concatenate(chunks, axis=0)
    mean = np.mean(taggram, axis=0)
    labels = config.MSD_LABELS
    top = []
    for index in mean.argsort()[-3:][::-1]:
        top.append(labels[index])
    return top


def tag_path(path, n_frames, overlap, big, reg):
    batch, _spec = batch_data(path, n_frames, overlap)
    if batch is None or getattr(batch, "shape", [0])[0] == 0:
        return [], []
    sess, x, is_training, y = big
    found_big = run_model(sess, x, is_training, y, batch)
    sess, x, is_training, y = reg
    found_reg = run_model(sess, x, is_training, y, batch)
    return list(found_big), list(found_reg)


def emit(payload):
    sys.stdout.write(json.dumps(payload))
    sys.stdout.write("\n")
    sys.stdout.flush()


def payload_for(path, n_frames, overlap, big, reg):
    try:
        found_big, found_reg = tag_path(path, n_frames, overlap, big, reg)
        return {"path": path, "big": found_big, "reg": found_reg}
    except Exception:
        return {"path": path, "big": [], "reg": [], "error": traceback.format_exc().splitlines()[-1]}


def load_models():
    n_frames = n_frames_for(3.0)
    overlap = n_frames
    big = load_session("MSD_musicnn_big", n_frames)
    reg = load_session("MSD_musicnn", n_frames)
    return n_frames, overlap, big, reg


def main(argv):
    batch = len(argv) >= 2 and argv[1] == "--batch"
    if not batch and len(argv) < 2:
        emit({"big": [], "reg": []})
        return 1
    n_frames, overlap, big, reg = load_models()
    if batch:
        while True:
            line = sys.stdin.readline()
            if not line:
                break
            path = line.strip()
            if not path:
                continue
            emit(payload_for(path, n_frames, overlap, big, reg))
        return 0
    path = argv[1]
    out = payload_for(path, n_frames, overlap, big, reg)
    emit({"big": out.get("big", []), "reg": out.get("reg", [])})
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
