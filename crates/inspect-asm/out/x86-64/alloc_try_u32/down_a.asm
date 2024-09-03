inspect_asm::alloc_try_u32::down_a:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	push rax
	mov r14, rsi
	mov rbx, rdi
	mov r15, qword ptr [rsi]
	mov r12, qword ptr [r15]
	lea r13, [r12 - 8]
	cmp r13, qword ptr [r15 + 8]
	jb .LBB0_0
	test r13, r13
	jne .LBB0_5
.LBB0_0:
	mov rdi, r15
	jmp .LBB0_2
.LBB0_1:
	mov rdi, rax
	test r13, r13
	jne .LBB0_5
.LBB0_2:
	mov rax, qword ptr [rdi + 24]
	test rax, rax
	je .LBB0_3
	mov qword ptr [rax], rax
	mov qword ptr [r14], rax
	mov r13, qword ptr [rax]
	add r13, -8
	cmp r13, qword ptr [rax + 8]
	jae .LBB0_1
	xor r13d, r13d
	jmp .LBB0_1
.LBB0_3:
	mov rbp, rdx
	mov esi, 4
	mov edx, 8
	call bump_scope::chunk_raw::RawChunk<_,A>::append_for
	mov qword ptr [r14], rax
	mov r13, qword ptr [rax]
	add r13, -8
	cmp r13, qword ptr [rax + 8]
	jae .LBB0_4
	xor r13d, r13d
.LBB0_4:
	mov rdx, rbp
.LBB0_5:
	call rdx
	mov dword ptr [r13], eax
	mov dword ptr [r13 + 4], edx
	test eax, eax
	je .LBB0_6
	mov qword ptr [r15], r12
	mov qword ptr [r14], r15
	mov dword ptr [rbx + 4], edx
	mov eax, 1
	jmp .LBB0_7
.LBB0_6:
	add r13, 4
	mov rax, r13
	and rax, -4
	mov rcx, qword ptr [r14]
	mov qword ptr [rcx], rax
	mov qword ptr [rbx + 8], r13
	xor eax, eax
.LBB0_7:
	mov dword ptr [rbx], eax
	mov rax, rbx
	add rsp, 8
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
