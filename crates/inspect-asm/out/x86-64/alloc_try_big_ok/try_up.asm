inspect_asm::alloc_try_big_ok::try_up:
	push rbp
	mov rbp, rsp
	push r15
	push r14
	push r13
	push r12
	push rbx
	and rsp, -512
	sub rsp, 2048
	mov r14, rsi
	mov rbx, rdi
	mov r13, qword ptr [rsi]
	mov rax, qword ptr [r13]
	mov qword ptr [rsp + 504], rax
	lea r15, [rax - 1]
	and r15, -512
	mov rcx, r15
	add rcx, 1536
	mov rax, -1
	cmovb rcx, rax
	cmp rcx, qword ptr [r13 + 8]
	ja .LBB0_0
	add r15, 512
	jne .LBB0_6
.LBB0_0:
	mov rdi, r13
	jmp .LBB0_2
.LBB0_1:
	add r15, 512
	mov rdi, rcx
	test r15, r15
	jne .LBB0_6
.LBB0_2:
	mov rcx, qword ptr [rdi + 24]
	test rcx, rcx
	je .LBB0_3
	lea rsi, [rcx + 32]
	mov qword ptr [rcx], rsi
	mov qword ptr [r14], rcx
	mov r15, qword ptr [rcx]
	dec r15
	and r15, -512
	mov rsi, r15
	add rsi, 1536
	cmovb rsi, rax
	cmp rsi, qword ptr [rcx + 8]
	jbe .LBB0_1
	xor r15d, r15d
	mov rdi, rcx
	test r15, r15
	je .LBB0_2
	jmp .LBB0_6
.LBB0_3:
	mov r12, rdx
	mov esi, 512
	mov edx, 1024
	call bump_scope::chunk_raw::RawChunk<_,A>::append_for
	test rax, rax
	je .LBB0_4
	mov qword ptr [r14], rax
	mov r15, qword ptr [rax]
	dec r15
	and r15, -512
	mov rcx, r15
	add rcx, 1536
	mov rdx, -1
	cmovae rdx, rcx
	cmp rdx, qword ptr [rax + 8]
	mov rdx, r12
	jbe .LBB0_5
	xor r15d, r15d
	jmp .LBB0_6
.LBB0_4:
	mov dword ptr [rbx], 2
	jmp .LBB0_9
.LBB0_5:
	add r15, 512
.LBB0_6:
	lea r12, [rsp + 512]
	mov rdi, r12
	call rdx
	mov edx, 1024
	mov rdi, r15
	mov rsi, r12
	call qword ptr [rip + memcpy@GOTPCREL]
	lea rcx, [r15 + 4]
	lea rax, [r15 + 512]
	cmp dword ptr [r15], 0
	cmovne rax, rcx
	je .LBB0_7
	mov ecx, dword ptr [rcx]
	mov rdx, qword ptr [rsp + 504]
	mov qword ptr [r13], rdx
	mov qword ptr [r14], r13
	mov edx, 1
	jmp .LBB0_8
.LBB0_7:
	add r15, 1024
	mov rcx, qword ptr [r14]
	mov qword ptr [rcx], r15
	xor edx, edx
.LBB0_8:
	mov dword ptr [rbx], edx
	mov dword ptr [rbx + 4], ecx
	mov qword ptr [rbx + 8], rax
.LBB0_9:
	mov rax, rbx
	lea rsp, [rbp - 40]
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
