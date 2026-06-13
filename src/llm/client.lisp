#|======================================================================
CLIENT.LISP

1. HTTP client for DeepSeek API — sends chat completion requests.
2. Called by: engine.lisp → client.lisp → dexador → https://api.deepseek.com/chat/completions
3. (2025-06-13) Phase 1: non-streaming + streaming API calls.

    Author: liangzid
    Copyright © 2025, all rights reserved.
    Created: 13 June 2025
======================================================================|#
(in-package :xgamengine)

;; ---- Configuration ----

(defvar *api-key* nil
  "DeepSeek API key. Set from environment variable DEEPSEEK_API_KEY
or by reading the key file directly.")

(defvar *api-base-url* "https://api.deepseek.com"
  "Base URL for the DeepSeek API.")

(defvar *model* "deepseek-v4-pro"
  "Model identifier for API calls.")

(defvar *api-timeout* 120
  "HTTP request timeout in seconds.")

;; ---- Helpers ----

(defun ensure-api-key ()
  "Return the API key, loading it from the environment or key file if needed.
Raises an error if no key is available."
  (or *api-key*
      (setf *api-key*
            (or (uiop:getenv "DEEPSEEK_API_KEY")
                (load-api-key-from-file)))
      (error "No DeepSeek API key found. Set DEEPSEEK_API_KEY or create a key file.")))

(defun load-api-key-from-file ()
  "Attempt to read the API key from ../deepseek-api-key.txt (relative to xgamengine root).
Returns the trimmed string or NIL."
  (let ((key-file (merge-pathnames "../deepseek-api-key.txt"
                                   (asdf:system-source-directory :xgamengine))))
    (when (uiop:file-exists-p key-file)
      (with-open-file (in key-file :direction :input)
        (string-trim '(#\Space #\Newline #\Return #\Tab)
                     (read-line in nil ""))))))

(defun build-chat-completion-url ()
  "Return the full chat completions endpoint URL."
  (concatenate 'string *api-base-url* "/chat/completions"))

(defun build-headers ()
  "Return an alist of HTTP headers for the API request."
  (list (cons "Content-Type" "application/json")
        (cons "Authorization" (concatenate 'string "Bearer " (ensure-api-key)))))

;; ---- JSON Encoding ----

(defun messages-to-json (messages)
  "Convert a list of message plists to a JSON array string.
Each message is a plist like (:role \"user\" :content \"Hello\")."
  (with-output-to-string (s)
    (write-string "[" s)
    (loop for msg in messages
          for first = t then nil
          do (unless first (write-string "," s))
             (write-string (json-encode-plist msg) s))
    (write-string "]" s)))

(defun build-request-body (messages &key (stream nil) (temperature nil)
                                         (max-tokens nil))
  "Build the JSON request body for a chat completion call.
Uses plists with keyword keys; json-encode handles lowercasing for the API.
Returns a JSON string."
  (let ((body-plist (list :model *model*
                          :messages messages
                          :thinking (list :|type| "disabled"))))
    (when stream
      (setf (getf body-plist :stream) t))
    (when temperature
      (setf (getf body-plist :temperature) temperature))
    (when max-tokens
      (setf (getf body-plist :max_tokens) max-tokens))
    (json-encode body-plist)))

;; ---- API Calls ----

(defun call-chat-completion (messages &key (stream nil) (temperature 0.9)
                                         (max-tokens 512))
  "Send a chat completion request to the DeepSeek API.
MESSAGES is a list of plists with keys :role and :content.
If STREAM is T, returns a function that yields SSE chunks.
Otherwise, blocks and returns the parsed JSON response as a plist.
TEMPERATURE defaults to 0.9 for creative narrative generation.
MAX-TOKENS defaults to 512."
  (let ((url (build-chat-completion-url))
        (headers (build-headers))
        (body (build-request-body messages
                                  :stream stream
                                  :temperature temperature
                                  :max-tokens max-tokens)))
    (if stream
        (call-chat-completion-streaming url headers body)
        (call-chat-completion-sync url headers body))))

(defun call-chat-completion-sync (url headers body)
  "Make a synchronous (non-streaming) API call.
Returns the parsed response body as a plist."
  (let ((response (dexador:post url
                                :headers headers
                                :content body
                                :connect-timeout 10
                                :read-timeout *api-timeout*)))
    (json-decode response)))

(defun call-chat-completion-streaming (url headers body)
  "Streaming API call — NOT YET IMPLEMENTED for MVP.
Returns a signal that streaming is unavailable; use non-streaming mode instead."
  (declare (ignore url headers body))
  (error "Streaming mode is not yet implemented. Use :stream nil."))

;; ---- Response Extraction ----

(defun extract-assistant-content (response-alist)
  "Extract the assistant's message content from a chat completion response.
RESPONSE-ALIST is the parsed JSON alist from the API."
  (json-get-in response-alist "choices" 0 "message" "content"))

(defun extract-usage (response-alist)
  "Extract token usage info from the response, returned as a plist."
  (let ((usage (json-get response-alist "usage")))
    (when usage
      (list :prompt-tokens (json-get usage "prompt_tokens")
            :completion-tokens (json-get usage "completion_tokens")
            :total-tokens (json-get usage "total_tokens")))))
