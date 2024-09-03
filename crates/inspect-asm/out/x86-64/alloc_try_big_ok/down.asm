inspect_asm::alloc_try_big_ok::down:
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
	mov r13, qword ptr [rsi]
	mov rax, qword ptr [r13]
	xor r15d, r15d
	mov qword ptr [rsp + 504], rax
	sub rax, 1024
	cmovae r15, rax
	mov rbx, rdi
	and r15, -512
	cmp r15, qword ptr [r13 + 8]
	jae .LBB0_2
	xor eax, eax
	mov rdi, r13
.LBB0_0:
	mov rcx, qword ptr [rdi + 24]
	test rcx, rcx
	je .LBB0_1
	mov qword ptr [rcx], rcx
	mov qword ptr [r14], rcx
	mov r15, qword ptr [rcx]
	sub r15, 1024
	cmovb r15, rax
	and r15, -512
	sete sil
	cmp r15, qword ptr [rcx + 8]
	setb dil
	or dil, sil
	mov rdi, rcx
	jne .LBB0_0
	jmp .LBB0_2
.LBB0_1:
	mov esi, 512
	mov r15, rdx
	mov edx, 1024
	call bump_scope::chunk_raw::RawChunk<_,A>::append_for
	mov rdx, r15
	mov qword ptr [r14], rax
	mov rax, qword ptr [rax]
	xor r15d, r15d
	sub rax, 1024
	cmovae r15, rax
	and r15, -512
.LBB0_2:
	lea r12, [rsp + 512]
	mov rdi, r12
	call rdx
	mov edx, 1024
	mov rdi, r15
	mov rsi, r12
	call qword ptr [rip + memcpy@GOTPCREL]
	test byte ptr [r15], 1
	je .LBB0_3
	mov eax, dword ptr [r15 + 4]
	mov rcx, qword ptr [rsp + 504]
	mov qword ptr [r13], rcx
	mov qword ptr [r14], r13
	mov dword ptr [rbx + 4], eax
	mov eax, 1
	jmp .LBB0_4
.LBB0_3:
	add r15, 512
	mov rax, qword ptr [r14]
	mov qword ptr [rax], r15
	mov qword ptr [rbx + 8], r15
	xor eax, eax
.LBB0_4:
	mov dword ptr [rbx], eax
	mov rax, rbx
	lea rsp, [rbp - 40]
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
