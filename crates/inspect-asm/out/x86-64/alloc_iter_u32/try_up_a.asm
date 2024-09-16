inspect_asm::alloc_iter_u32::try_up_a:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	test rdx, rdx
	je .LBB0_1
	mov rax, rdx
	shr rax, 61
	je .LBB0_2
.LBB0_0:
	xor eax, eax
	jmp .LBB0_8
.LBB0_1:
	mov eax, 4
	xor edx, edx
	xor ecx, ecx
	jmp .LBB0_7
.LBB0_2:
	mov rbx, rsi
	lea r15, [4*rdx]
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rsi, qword ptr [rcx + 8]
	sub rsi, rax
	cmp r15, rsi
	ja .LBB0_12
	lea rsi, [r15 + rax]
	mov qword ptr [rcx], rsi
.LBB0_3:
	mov qword ptr [rsp], rax
	mov qword ptr [rsp + 8], 0
	mov qword ptr [rsp + 16], rdx
	mov qword ptr [rsp + 24], rdi
	xor r12d, r12d
	mov r14, rsp
	xor edx, edx
	jmp .LBB0_5
.LBB0_4:
	mov dword ptr [rax + 4*rdx], ebp
	inc rdx
	mov qword ptr [rsp + 8], rdx
	add r12, 4
	cmp r15, r12
	je .LBB0_6
.LBB0_5:
	mov ebp, dword ptr [rbx + r12]
	cmp qword ptr [rsp + 16], rdx
	jne .LBB0_4
	mov rdi, r14
	call bump_scope::bump_vec::BumpVec<T,A,_,_,_>::generic_grow_cold
	test al, al
	jne .LBB0_11
	mov rax, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 8]
	jmp .LBB0_4
.LBB0_6:
	mov rax, qword ptr [rsp]
	mov rcx, qword ptr [rsp + 16]
	mov rdi, qword ptr [rsp + 24]
	shl rcx, 2
.LBB0_7:
	lea r9, [rax + rcx]
	mov rsi, qword ptr [rdi]
	mov r8, qword ptr [rsi]
	cmp r9, r8
	je .LBB0_9
	add rcx, rax
	cmp rcx, r8
	je .LBB0_10
.LBB0_8:
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_9:
	lea rcx, [4*rdx]
	lea r8, [rax + 4*rdx]
	add r8, 3
	and r8, -4
	mov qword ptr [rsi], r8
	mov qword ptr [rsp + 16], rdx
	mov rsi, qword ptr [rdi]
	mov r8, qword ptr [rsi]
	add rcx, rax
	cmp rcx, r8
	jne .LBB0_8
.LBB0_10:
	lea rcx, [rax + 4*rdx]
	add rcx, 3
	and rcx, -4
	mov qword ptr [rsi], rcx
	jmp .LBB0_8
.LBB0_11:
	mov rax, qword ptr [rsp]
	mov rcx, qword ptr [rsp + 16]
	lea rdx, [rax + 4*rcx]
	mov rcx, qword ptr [rsp + 24]
	mov rcx, qword ptr [rcx]
	cmp rdx, qword ptr [rcx]
	jne .LBB0_0
	add rax, 3
	and rax, -4
	mov qword ptr [rcx], rax
	jmp .LBB0_0
.LBB0_12:
	mov r14, rdi
	mov rsi, rdx
	mov r12, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rdx, r12
	mov rdi, r14
	test rax, rax
	jne .LBB0_3
	jmp .LBB0_0
	mov rcx, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 16]
	lea rsi, [rcx + 4*rdx]
	mov rdx, qword ptr [rsp + 24]
	mov rdx, qword ptr [rdx]
	cmp rsi, qword ptr [rdx]
	jne .LBB0_13
	add rcx, 3
	and rcx, -4
	mov qword ptr [rdx], rcx
.LBB0_13:
	mov rdi, rax
	call _Unwind_Resume@PLT
