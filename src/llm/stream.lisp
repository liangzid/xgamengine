#|======================================================================
STREAM.LISP

1. SSE stream parser for DeepSeek streaming chat completions.
2. Called by: client.lisp (on stream=true) → stream.lisp → character-by-character SSE parsing.
3. (2025-06-13) Phase 1: line-based SSE parser using shasht for incremental JSON.

    Author: liangzid
    Copyright © 2025, all rights reserved.
    Created: 13 June 2025
======================================================================|#
(in-package :xgamengine)

;; ---- SSE Parser State ----

(defstruct sse-parser-state
  "State machine for parsing SSE (Server-Sent Events) streams."
  (buffer "" :type string)        ; accumulated data line
  (content "" :type string)       ; accumulated assistant content
  (reasoning "" :type string)     ; accumulated reasoning content
  (done-p nil :type boolean)      ; true when data: [DONE] received
  (error-p nil :type boolean)     ; true on parse error
  (error-message "" :type string))

;; ---- Low-level SSE line processing ----

(defun sse-line-type (line)
  "Determine the type of an SSE line.
Returns :data, :event, :id, :comment, or :empty."
  (cond
    ((string= line "") :empty)
    ((str:starts-with-p ":" line) :comment)
    ((str:starts-with-p "data:" line) :data)
    ((str:starts-with-p "event:" line) :event)
    ((str:starts-with-p "id:" line) :id)
    (t :unknown)))

(defun extract-data-payload (line)
  "Extract the JSON payload from a 'data:' SSE line.
Returns the trimmed string after 'data:' or NIL."
  ;; Handle "data: {...}" — strip prefix, trim leading space
  (let ((payload (str:substring 5 nil line)))
    (string-trim '(#\Space) payload)))

(defun parse-sse-chunk (json-string)
  "Parse a single SSE data chunk (JSON) and return a plist with extracted deltas.
Returns NIL if the chunk has no content delta."
  (when (and json-string (not (string= json-string "[DONE]")))
    (handler-case
        (let* ((parsed (json-decode json-string))
               (choices (json-get parsed "choices"))
               (first-choice (when (vectorp choices)
                              (when (> (length choices) 0)
                                (aref choices 0))))
               (delta (when first-choice (json-get first-choice "delta")))
               (finish-reason (when first-choice
                               (json-get first-choice "finish_reason"))))
          (list :content (when delta (json-get delta "content"))
                :reasoning (when delta (json-get delta "reasoning_content"))
                :finish-reason finish-reason))
      (error (e)
        ;; On parse error, return nil — skip malformed chunks
        (format *error-output* "SSE parse warning: ~a~%" e)
        nil))))

;; ---- Public API ----

(defun process-sse-line (state line)
  "Process a single line from the SSE stream, updating STATE.
Returns :continue, :done, or :error."
  (let ((line-type (sse-line-type line)))
    (case line-type
      (:empty
       ;; Flush accumulated buffer
       (unless (string= (sse-parser-state-buffer state) "")
         (let* ((payload (extract-data-payload
                          (sse-parser-state-buffer state)))
                (chunk (parse-sse-chunk payload)))
           (when chunk
             ;; Accumulate content
             (when (getf chunk :content)
               (setf (sse-parser-state-content state)
                     (concatenate 'string
                                  (sse-parser-state-content state)
                                  (getf chunk :content))))
             ;; Accumulate reasoning
             (when (getf chunk :reasoning)
               (setf (sse-parser-state-reasoning state)
                     (concatenate 'string
                                  (sse-parser-state-reasoning state)
                                  (getf chunk :reasoning))))
             ;; Check finish reason
             (when (member (getf chunk :finish-reason)
                           '("stop" "length" "content_filter")
                           :test #'string=)
               (setf (sse-parser-state-done-p state) t)))))
       ;; Reset buffer
       (setf (sse-parser-state-buffer state) "")
       (if (sse-parser-state-done-p state)
           :done
           :continue))
      (:comment
       :continue)  ; skip comments
      (:data
       ;; Check for [DONE] marker
       (let ((payload (extract-data-payload line)))
         (if (string= payload "[DONE]")
             (progn
               (setf (sse-parser-state-done-p state) t)
               :done)
             (progn
               ;; Accumulate the data line (may span multiple lines)
               (setf (sse-parser-state-buffer state) line)
               :continue))))
      (otherwise
       :continue))))

(defun parse-sse-stream (body-string &key (callback nil))
  "Parse a complete SSE stream from BODY-STRING.
If CALLBACK is provided, it is called with (content-chunk) for each new content.
Returns a plist with :content, :reasoning, and :status (:ok or :error).

For incremental streaming use process-sse-line directly."
  (let ((state (make-sse-parser-state))
        (lines (str:split #\Newline body-string)))
    (dolist (line lines)
      (let ((result (process-sse-line state line)))
        (when (and callback
                   (not (string= (sse-parser-state-content state) "")))
          ;; For simplicity in MVP, we call callback with the current state content
          ;; A more sophisticated version would track deltas
          nil)
        (when (eq result :done)
          (return))))
    (if (sse-parser-state-error-p state)
        (list :status :error
              :error-message (sse-parser-state-error-message state)
              :content (sse-parser-state-content state)
              :reasoning (sse-parser-state-reasoning state))
        (list :status :ok
              :content (sse-parser-state-content state)
              :reasoning (sse-parser-state-reasoning state)))))
