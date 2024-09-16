inspect_asm::alloc_iter_u32::try_down_a:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	sub rsp, 40
	mov r14, rdi
	test rdx, rdx
	je .LBB0_1
	mov rax, rdx
	shr rax, 61
	je .LBB0_2
.LBB0_0:
	xor eax, eax
	jmp .LBB0_15
.LBB0_1:
	mov qword ptr [rsp + 24], rdx
	mov esi, 4
	xor ebx, ebx
	jmp .LBB0_7
.LBB0_2:
	mov r15, rsi
	lea r12, [4*rdx]
	mov rcx, qword ptr [r14]
	mov rax, qword ptr [rcx]
	mov rsi, rax
	sub rsi, qword ptr [rcx + 8]
	cmp r12, rsi
	ja .LBB0_17
	sub rax, r12
	mov qword ptr [rcx], rax
	je .LBB0_17
.LBB0_3:
	mov qword ptr [rsp + 8], rax
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdx
	mov qword ptr [rsp + 32], r14
	xor r13d, r13d
	lea r14, [rsp + 8]
	xor ebx, ebx
	jmp .LBB0_5
.LBB0_4:
	mov dword ptr [rax + 4*rbx], ebp
	inc rbx
	mov qword ptr [rsp + 16], rbx
	add r13, 4
	cmp r12, r13
	je .LBB0_6
.LBB0_5:
	mov ebp, dword ptr [r15 + r13]
	cmp qword ptr [rsp + 24], rbx
	jne .LBB0_4
	mov rdi, r14
	call bump_scope::bump_vec::BumpVec<T,A,_,_,_>::generic_grow_cold
	test al, al
	jne .LBB0_16
	mov rax, qword ptr [rsp + 8]
	mov rbx, qword ptr [rsp + 16]
	jmp .LBB0_4
.LBB0_6:
	mov rsi, qword ptr [rsp + 8]
	mov r14, qword ptr [rsp + 32]
.LBB0_7:
	lea r15, [4*rbx]
	mov rax, qword ptr [r14]
	mov rax, qword ptr [rax]
	cmp rsi, rax
	je .LBB0_9
	mov r12, rsi
	cmp r12, rax
	je .LBB0_12
.LBB0_8:
	mov rax, r12
	jmp .LBB0_15
.LBB0_9:
	mov rax, qword ptr [rsp + 24]
	lea rax, [rsi + 4*rax]
	xor r12d, r12d
	sub rax, r15
	cmovae r12, rax
	and r12, -4
	lea rax, [r15 + rsi]
	mov rdi, r12
	mov rdx, r15
	cmp rax, r12
	jbe .LBB0_10
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_11
.LBB0_10:
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_11:
	mov rax, qword ptr [r14]
	mov qword ptr [rax], r12
	mov qword ptr [rsp + 24], rbx
	mov rax, qword ptr [r14]
	mov rax, qword ptr [rax]
	cmp r12, rax
	jne .LBB0_8
.LBB0_12:
	mov rcx, qword ptr [rsp + 24]
	lea rcx, [rax + 4*rcx]
	xor edx, edx
	sub rcx, r15
	cmovae rdx, rcx
	and rdx, -4
	lea rcx, [r15 + rax]
	mov rdi, rdx
	sub rdi, rax
	add rdi, r12
	mov r13, rdi
	mov rsi, r12
	cmp rcx, rdx
	jbe .LBB0_13
	mov rdx, r15
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_14
.LBB0_13:
	mov rdx, r15
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_14:
	mov rcx, qword ptr [r14]
	mov rax, r13
	mov qword ptr [rcx], r13
.LBB0_15:
	mov rdx, rbx
	add rsp, 40
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_16:
	mov rcx, qword ptr [rsp + 8]
	mov rax, qword ptr [rsp + 32]
	mov rax, qword ptr [rax]
	cmp qword ptr [rax], rcx
	jne .LBB0_0
	mov rdx, qword ptr [rsp + 24]
	lea rcx, [rcx + 4*rdx]
	add rcx, 3
	and rcx, -4
	mov qword ptr [rax], rcx
	jmp .LBB0_0
.LBB0_17:
	mov rdi, r14
	mov rsi, rdx
	mov rbx, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rdx, rbx
	test rax, rax
	jne .LBB0_3
	jmp .LBB0_0
	mov rdx, qword ptr [rsp + 8]
	mov rcx, qword ptr [rsp + 32]
	mov rcx, qword ptr [rcx]
	cmp qword ptr [rcx], rdx
	jne .LBB0_18
	mov rsi, qword ptr [rsp + 24]
	lea rdx, [rdx + 4*rsi]
	add rdx, 3
	and rdx, -4
	mov qword ptr [rcx], rdx
.LBB0_18:
	mov rdi, rax
	call _Unwind_Resume@PLT
